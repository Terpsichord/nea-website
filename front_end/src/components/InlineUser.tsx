import { Link } from "react-router";
import { InlineUser } from "../types";

export interface InlineUserProps {
    user: InlineUser;
    small?: boolean;
}

function InlineUserView({ user, small }: InlineUserProps) {
    return (
        <Link to={`/user/${user.username}`} className="text-lg">
            <img src={user.pictureUrl} draggable={false} className={`rounded-full inline ${small ? "size-7 mr-2" : "size-10 mr-3" }`} />
            {user.username}
        </Link>
    );
}

export default InlineUserView;