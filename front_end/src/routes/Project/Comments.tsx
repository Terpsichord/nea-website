import { useState } from "react";
import { useAuth } from "../../auth";
import TextArea from "../../components/TextArea";
import { ProjectComment } from "../../types";
import { fetchApi, useApi } from "../../utils";
import CommentView from "./CommentView";
import Loading from "../../components/Loading";

function Comments({ project }: { project: { username: string, id: string } }) {
    const { isAuth } = useAuth();

    const [id, setId] = useState(0);
    const refreshComments = () => setId(Math.random());

    const [comments] = useApi<ProjectComment[]>(`/project/${project.username}/${project.id}/comments`, { deps: [id] });

    const maxCommentLength = 100;
    async function submitComment(contents: string) {
        await fetchApi(`/project/${project.username}/${project.id}/comment`, {
            method: "POST",
            headers: {
                "Content-Type": "application/json",
            },
            body: JSON.stringify({ contents, parent_id: null })
        });

        refreshComments();
    }

    return (
        <div>
            <h3 className="text-2xl font-medium mt-8 mb-2">Comments</h3>
            {isAuth && <TextArea key={id} maxLength={maxCommentLength} submitText="Post Comment" onSubmit={submitComment} className="px-2 py-1 bg-blue-gray" submitClass="text-gray" />}
            <div className="space-y-8 mb-12">
                { comments === undefined ?
                    <Loading /> :
                    comments.map(comment => <CommentView {...comment} />)
                }
            </div>
        </div>
    )
}

export default Comments;