import { ProjectComment } from "../../types";
import CommentView from "./CommentView";

function Comments({ comments }: { comments: ProjectComment[] }) {
    return (
        <div>
            <h3 className="text-2xl font-medium mt-8 mb-2">Comments</h3>
            <div className="space-y-8 mb-12">
                {comments.map(comment => <CommentView {...comment} />)}

            </div>
        </div>
    )
}

export default Comments;