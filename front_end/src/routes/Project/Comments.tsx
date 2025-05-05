import { useRef, useState } from "react";
import { useAuth } from "../../auth";
import TextArea from "../../components/TextArea";
import { ProjectComment } from "../../types";
import { fetchApi, useApi } from "../../utils";
import CommentView from "./CommentView";
import Loading from "../../components/Loading";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faXmark } from "@fortawesome/free-solid-svg-icons";

function Comments({ project }: { project: { username: string, id: string } }) {
    const { isAuth } = useAuth();

    const [id, setId] = useState(0);
    const refreshComments = () => setId(Math.random());

    const [replyingTo, setReplyingTo] = useState<ProjectComment | null>(null);

    const [comments] = useApi<ProjectComment[]>(`/project/${project.username}/${project.id}/comments`, { deps: [id] });

    const maxCommentLength = 100;
    async function submitComment(contents: string) {
        await fetchApi(`/project/${project.username}/${project.id}/comment`, {
            method: "POST",
            headers: {
                "Content-Type": "application/json",
            },
            body: JSON.stringify({ contents, parent_id: replyingTo?.id ?? null })
        });

        refreshComments();
    }

    const textAreaRef = useRef<HTMLTextAreaElement | null>(null);

    function onReply(comment: ProjectComment) {
        setReplyingTo(comment);
        textAreaRef.current?.focus();
    }

    return (
        <div>
            <h3 className="text-2xl font-medium mt-8 mb-2">Comments</h3>
            {isAuth && <>
                <TextArea ref={textAreaRef} key={id} maxLength={maxCommentLength} submitText="Post Comment" onSubmit={submitComment} className="px-2 py-1 bg-blue-gray" submitClass="text-gray"
                    subtext={
                        replyingTo ?
                            <div className="mb-4 text-gray">
                                Replying to {replyingTo.user.username}: <i className="truncate">{replyingTo.contents}</i>
                                <FontAwesomeIcon icon={faXmark} className="ml-2" onClick={() => setReplyingTo(null)} />
                            </div> :
                            undefined
                    } />
            </>}
            <div className="space-y-8 mb-12">
                {comments === undefined ?
                    <Loading /> :
                    comments.map(comment => <CommentView comment={comment} onReply={isAuth ? onReply : null} />)

                }
            </div>
        </div>
    )
}

export default Comments;