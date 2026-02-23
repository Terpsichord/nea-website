import numpy as np
from core import l2_normalize, bpr_loss, top_k_indices
from user import UserTower
from item import ItemTower


class TwinTowerModel:
    def __init__(self, num_users, num_items, emb_dim=64, hidden_layer_sizes=(64,)):
        self.user_tower = UserTower(num_users, emb_dim, hidden_layer_sizes)
        self.item_tower = ItemTower(num_items, emb_dim, 20, 16, 16, hidden_layer_sizes)
        self.item_matrix = None

    def precompute_item_matrix(self, item_records):
        _, matrix = self.item_tower.precompute_matrix(item_records)
        self.item_matrix = l2_normalize(matrix, axis=1)

    def recommend(self, user_id, user_history_ids, top_k=10):
        if self.item_matrix is None:
            raise ValueError("Item matrix must be precomputed for recommendation.")

        u_emb = l2_normalize(
            self.user_tower.forward([user_id], [user_history_ids], self.item_matrix)
        )
        scores = u_emb @ self.item_matrix.T
        indices = top_k_indices(scores[0], top_k)
        return indices, scores[0][indices]

    def _train_batch(self, batch, lr):
        # batch: (u_ids, histories, pos_items, neg_items)
        # items are dicts with: {'ids': [], 'tags': [[]], 'langs': []}
        u_ids, histories, pos, neg = batch

        # forward pass (dynamic for both to allow gradient flow)
        u_embs = l2_normalize(
            self.user_tower.forward(u_ids, histories, self.item_matrix)
        )
        p_embs = l2_normalize(
            self.item_tower.forward(pos["ids"], pos["tags"], pos["langs"])
        )
        n_embs = l2_normalize(
            self.item_tower.forward(neg["ids"], neg["tags"], neg["langs"])
        )

        # calculate bpr loss
        pos_scores = np.sum(u_embs * p_embs, axis=1)
        neg_scores = np.sum(u_embs * n_embs, axis=1)
        loss, grad_loss = bpr_loss(pos_scores - neg_scores, np.ones_like(pos_scores))

        # calculating gradients
        grad_loss = grad_loss.reshape(-1, 1)
        grad_u = grad_loss * (p_embs - n_embs)
        grad_p = grad_loss * u_embs
        grad_n = -grad_loss * u_embs

        # updating the model via backpropagation
        self.user_tower.update(grad_u, lr, u_ids)
        self.item_tower.update(grad_p, lr, pos["ids"], pos["tags"], pos["langs"])
        self.item_tower.update(grad_n, lr, neg["ids"], neg["tags"], neg["langs"])

        return loss

    def train(self, training_data, epochs=10, lr=0.01):
        # training_data should be a list of batches
        for epoch in range(epochs):
            total_loss = 0
            for batch in training_data:
                total_loss += self._train_batch(batch, lr)

            print(
                f"Epoch {epoch} | Average Loss: {total_loss / len(training_data):.4f}"
            )
