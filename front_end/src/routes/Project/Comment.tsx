import InlineUserView from "../../components/InlineUser";
import { ProjectComment } from "../../types";

function CommentView({ user, contents, children }: ProjectComment) {
    return (
        <div>
            <InlineUserView small user={user} />
            <div className="mt-2 w-fit rounded-xl bg-blue-gray py-1 px-3">{contents}</div>
            {children && children.length > 0 &&
                <div className="flex mt-2">
                    <span className="w-1 rounded-full ml-3 mr-5 bg-gray" />
                    <div className="mt-1 space-y-4">
                        {children.map(child => <CommentView {...child} />)}
                    </div>
                </div>
            }
        </div>
    );
}

export default CommentView;