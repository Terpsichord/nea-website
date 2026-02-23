import numpy as np


def relu(x):
    return np.maximum(0, x)


def relu_grad(x):
    return (x > 0).astype(np.float32)


def sigmoid(x):
    return 1 / (1 + np.exp(-np.clip(x, -15, 15)))


def l2_normalize(x, axis=-1, eps=1e-12):
    norm = np.linalg.norm(x, axis=axis, keepdims=True)
    return x / (norm + eps)


def bpr_loss(diff, weights):
    # diff: (batch_size,)
    # weights: (batch_size,)
    sig = sigmoid(diff)
    loss = -np.mean(weights * np.log(sig + 1e-8))
    # Gradient of loss w.r.t diff
    grad = -weights * (1 - sig) / len(diff)
    return loss, grad


def top_k_indices(scores, k):
    if scores.ndim == 1:
        return np.argsort(-scores)[:k]
    return np.argsort(-scores, axis=1)[:, :k]


class Embedding:
    def __init__(self, num_embeddings, dimension):
        self.weights = np.random.normal(
            scale=0.01, size=(num_embeddings, dimension)
        ).astype(np.float32)
        self.dim = dimension

    def get_embeddings(self, ids):
        return self.weights[np.array(ids, dtype=np.int32)]

    def backward(self, ids, grad, learning_rate):
        # np.add.at handles duplicate IDs in a batch by accumulating gradients
        np.add.at(self.weights, ids, -learning_rate * grad)


class MLP:
    def __init__(self, layer_sizes):
        self.weights = []
        self.biases = []
        for i in range(len(layer_sizes) - 1):
            w = (
                np.random.normal(
                    scale=np.sqrt(2.0 / layer_sizes[i]),
                    size=(layer_sizes[i], layer_sizes[i + 1]),
                ).astype(np.float32)
                * 0.1
            )

            self.weights.append(w)
            self.biases.append(np.zeros(layer_sizes[i + 1], dtype=np.float32))
        self.cache = []

    def forward_pass(self, x):
        self.cache = []
        for i, (w, b) in enumerate(zip(self.weights, self.biases)):
            input_val = x
            pre_activation = x @ w + b
            self.cache.append((input_val, pre_activation))
            if i < len(self.weights) - 1:
                x = relu(pre_activation)
            else:
                x = pre_activation  # Linear output for the final layer
        return x

    def backward_pass(self, grad, learning_rate):
        for i in reversed(range(len(self.weights))):
            input_val, pre_act = self.cache[i]
            if i < len(self.weights) - 1:
                grad = grad * relu_grad(pre_act)

            dw = input_val.T @ grad
            db = np.sum(grad, axis=0)

            next_grad = grad @ self.weights[i].T

            self.weights[i] -= learning_rate * dw
            self.biases[i] -= learning_rate * db
            grad = next_grad
        return grad
