import { useState } from "react";
import InlineUserView from "../../components/InlineUser";
import { ProjectComment } from "../../types";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faEllipsisH, faReply } from "@fortawesome/free-solid-svg-icons";

function CommentView({ comment, onReply }: { comment: ProjectComment, onReply: ((comment: ProjectComment) => void) | null }) {
    const { user, contents, children } = comment;

    const [isCollapsed, setCollapsed] = useState(true);

    const toggle = () => setCollapsed(!isCollapsed);
    return (
        <div>
            <InlineUserView small user={user} />
            <div>
                <div className="whitespace-pre-wrap inline-block mt-2 w-fit rounded-xl bg-blue-gray py-1 px-3" onClick={toggle}>{contents}</div>
                {onReply && <FontAwesomeIcon icon={faReply} className="mt-auto inline-block opacity-30 hover:opacity-100 ml-2" onClick={() => onReply(comment)} />}
            </div>
            {children && children.length > 0 &&
                <div className="mt-2">
                    {isCollapsed && <FontAwesomeIcon icon={faEllipsisH} onClick={toggle} className="ml-5" />}
                    <div className={isCollapsed ? "hidden" : "flex"}>
                        <span className="w-1 rounded-full ml-3 mr-5 bg-gray" />
                        <div className="mt-1 space-y-4">
                            {children.map(child => <CommentView comment={child} onReply={onReply} />)}
                        </div>
                    </div>
                </div>
            }
        </div>
    );
}

export default CommentView;