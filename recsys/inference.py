import pickle
import psycopg2
from psycopg2.extras import execute_values
import numpy as np
import sys
import os
from dotenv import load_dotenv
from urllib.parse import urlsplit

load_dotenv()
DB_URL = os.getenv("DATABASE_URL")

CATEGORY_ID = 1  # e.g., 'AI_TWIN_TOWER' in rec_category table


def run_recommendations(user_id, top_k):
    # load model and mappings from file
    with open("recommender.pkl", "rb") as f:
        data = pickle.load(f)
    model = data["model"]
    idx_to_project = data["idx_to_project"]
    project_to_idx = data["project_to_idx"]

    # get user history
    params = urlsplit(DB_URL)
    conn = psycopg2.connect(
        database = params.path[1:],
        user = params.username,
        password = params.password,
        host = params.hostname,
        port = params.port,
    )
    cur = conn.cursor()

    cur.execute("SELECT project_id FROM interactions WHERE user_id = %s", (user_id,))
    raw_history = [row[0] for row in cur.fetchall()]
    # convert db project_ids to model indices
    mapped_history = [
        project_to_idx[pid] for pid in raw_history if pid in project_to_idx
    ]

    # generate recommendations
    indices, scores = model.recommend(user_id, mapped_history, top_k=top_k)

    # save to table 'recs'
    insert_data = []
    for idx, score in zip(indices, scores):
        insert_data.append((user_id, idx_to_project[idx], CATEGORY_ID, float(score)))

    query = """
            INSERT INTO recs (user_id, project_id, category_id, score, created_at)
            VALUES %s \
            """
    execute_values(cur, query, insert_data, template="(%s, %s, %s, %s, NOW())")

    conn.commit()
    cur.close()
    conn.close()
    print(f"Saved {top_k} recommendations for user {user_id}")


if __name__ == "__main__":
    user_id = sys.argv[1]
    run_recommendations(user_id, top_k=10)
