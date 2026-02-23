import psycopg2
from psycopg2.extras import execute_values
import pickle
import numpy as np
import os
from dotenv import load_dotenv
from urllib.parse import urlsplit
from model import TwinTowerModel

load_dotenv()
DB_URL = os.getenv("DATABASE_URL")

LANGUAGE_LIST = [
    "py",
    "js",
    "ts",
    "rs",
    "c",
    "cpp",
    "cs",
    "sh",
    "java",
]

def fetch_data():
    params = urlsplit(DB_URL)
    print(params)
    conn = psycopg2.connect(
        database = params.path[1:],
        user = params.username,
        password = params.password,
        host = params.hostname,
        port = params.port,
    )
    cur = conn.cursor()

    # map projects to contiguous indices
    cur.execute("SELECT id, lang, tag_ids FROM projects")
    project_rows = cur.fetchall()
    project_to_idx = {row[0]: i for i, row in enumerate(project_rows)}
    idx_to_project = {i: row[0] for i, row in enumerate(project_rows)}

    item_records = []
    for row in project_rows:
        item_records.append(
            {
                "item_id": project_to_idx[row[0]],
                "tag_ids": row[2] if row[2] else [],
                "language_id": row[1] if row[1] else 0,
            }
        )

    # map users and fetch interactions
    cur.execute(
        "SELECT user_id, project_id FROM interactions WHERE type IN ('view', 'like')"
    )
    interactions = cur.fetchall()

    # group by user for history
    user_histories = {}
    for u_id, p_id in interactions:
        if p_id in project_to_idx:
            user_histories.setdefault(u_id, []).append(project_to_idx[p_id])

    cur.close()
    conn.close()
    return item_records, user_histories, project_to_idx, idx_to_project


def create_batches(user_histories, item_records, project_to_idx, batch_size=32):
    batches = []
    user_ids = list(user_histories.keys())
    all_item_ids = list(project_to_idx.values())

    for i in range(0, len(user_ids), batch_size):
        batch_u = user_ids[i : i + batch_size]

        u_ids, histories, pos, neg = (
            [],
            [],
            {"ids": [], "tags": [], "langs": []},
            {"ids": [], "tags": [], "langs": []},
        )

        for u in batch_u:
            # Positive: something they actually liked
            p_idx = np.random.choice(user_histories[u])
            # Negative: random item they haven't seen
            n_idx = np.random.choice(all_item_ids)

            u_ids.append(u)
            histories.append(user_histories[u])

            # Populate item data
            for target, idx in [(pos, p_idx), (neg, n_idx)]:
                target["ids"].append(idx)
                target["tags"].append(item_records[idx]["tag_ids"])
                target["langs"].append(item_records[idx]["language_id"])

        batches.append((np.array(u_ids), histories, pos, neg))
    return batches


if __name__ == "__main__":
    print("Fetching data from Postgres...")
    items, histories, p_to_idx, idx_to_p = fetch_data()

    model = TwinTowerModel(num_users=max(histories.keys()) + 1, num_items=len(items))
    model.precompute_item_matrix(items)

    print("Generating training batches...")
    batches = create_batches(histories, items, p_to_idx)

    print("Training...")
    model.train(batches, epochs=10, lr=0.01)

    # Save model + mapping
    payload = {"model": model, "idx_to_project": idx_to_p, "project_to_idx": p_to_idx}
    with open("recommender.pkl", "wb") as f:
        pickle.dump(payload, f)
    print("Success: Model and Mappings saved.")
