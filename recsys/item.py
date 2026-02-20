import numpy as np

class ItemTower:
    def __init__(self, num_items, item_emb_dim, num_tags, tag_emb_dim, lang_emb_dim, hidden_layer_sizes=(64,)):
        from core import Embedding, MLP
        self.item_id_emb = Embedding(num_items, item_emb_dim)
        self.tag_emb = Embedding(num_tags, tag_emb_dim)
        self.lang_emb = Embedding(10, lang_emb_dim) # 0-9 for languages

        self.feature_mlp = MLP([tag_emb_dim + lang_emb_dim, *hidden_layer_sizes, item_emb_dim])

    def forward(self, item_ids, tag_id_lists, lang_ids):
        id_embs = self.item_id_emb.get_embeddings(item_ids)

        t_embs = []
        for tags in tag_id_lists:
            if len(tags) > 0:
                t_embs.append(np.mean(self.tag_emb.get_embeddings(tags), axis=0))
            else:
                t_embs.append(np.zeros(self.tag_emb.dim))

        t_embs = np.array(t_embs, dtype=np.float32)
        l_embs = self.lang_emb.get_embeddings(lang_ids)

        feat_input = np.concatenate([t_embs, l_embs], axis=1)
        feat_output = self.feature_mlp.forward_pass(feat_input)

        return id_embs + feat_output

    def precompute_matrix(self, item_records):
        ids = [r['item_id'] for r in item_records]
        tags = [r['tag_ids'] for r in item_records]
        langs = [r['language_id'] for r in item_records]
        return ids, self.forward(ids, tags, langs)

    def update(self, grad, lr, item_ids, tag_id_lists, lang_ids):
        # update id embeddings
        self.item_id_emb.backward(item_ids, grad, lr)

        # backprop through mlp for features
        mlp_grad = self.feature_mlp.backward_pass(grad, lr)

        # update language and tags
        tag_dim = self.tag_emb.dim
        t_grad = mlp_grad[:, :tag_dim]
        l_grad = mlp_grad[:, tag_dim:]

        self.lang_emb.backward(lang_ids, l_grad, lr)

        for i, tags in enumerate(tag_id_lists):
            if len(tags) > 0:
                # distribute the mean gradient back to individual tags
                unit_t_grad = np.tile(t_grad[i] / len(tags), (len(tags), 1))
                self.tag_emb.backward(tags, unit_t_grad, lr)