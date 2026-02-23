import numpy as np
from core import l2_normalize


class UserTower:
    def __init__(self, num_users, emb_dim, hidden_layer_sizes=(64,)):
        from core import Embedding, MLP

        self.user_id_emb = Embedding(num_users, emb_dim)
        self.user_mlp = MLP([2 * emb_dim, *hidden_layer_sizes, emb_dim])

    def forward(self, user_ids, history_item_ids, item_matrix):
        u_emb = self.user_id_emb.get_embeddings(user_ids)

        # compute history embedding as mean of previously interacted item embeddings
        h_embs = []
        for hist in history_item_ids:
            if len(hist) > 0:
                h_embs.append(np.mean(item_matrix[hist], axis=0))
            else:
                h_embs.append(np.zeros(item_matrix.shape[1]))
        h_embs = np.array(h_embs, dtype=np.float32)

        combined = np.concatenate([u_emb, h_embs], axis=1)
        return self.user_mlp.forward_pass(combined)

    def update(self, grad, lr, user_ids):
        mlp_grad = self.user_mlp.backward_pass(grad, lr)
        self.user_id_emb.backward(user_ids, mlp_grad[:, : self.user_id_emb.dim], lr)
