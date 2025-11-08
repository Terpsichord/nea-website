### Check if a user is followed

```sql
SELECT EXISTS (
  SELECT 1 FROM follows f
  INNER JOIN users u1 ON f.follower_id = u1.id
  INNER JOIN users u2 ON f.followee_id = u2.id
  WHERE u1.github_id = $1
  AND u2.username = $2
) AS "follows!";
```
This query checks whether an authenticated user with the given GitHub ID (`$1`) follows another user (`$2`).

The `user` table is joined onto `follows` twice:
 - `u1` represents the follower, matched by `u1.github_id = $1` 
 - `u2` represents the followee, matched by `u2.username = $2`

`SELECT EXISTS (...)` returns a boolean, which is true if the record exists (and therefore the given user is followed by the authenticated user) and false otherwise.

### Follow a user

```sql
INSERT INTO follows (follower_id, followee_id)
SELECT 
  (SELECT id FROM users WHERE github_id = $1),
  (SELECT id FROM users WHERE username = $2);
```

Adds a new record into the `follows` table, indicating that the authenticated user with the given GitHub ID (`$1`) follows the user with the given username (`$2`).

### Unfollow a user

```sql
DELETE FROM follows
WHERE follower_id = (SELECT id FROM users WHERE github_id = $1)
  AND followee_id = (SELECT id FROM users WHERE username = $2);
```

Removes the record that indicates the authenticated user with the given GitHub ID (`$1`) follows the user with the given username (`$2`).

