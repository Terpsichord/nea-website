import { Link } from "react-router";

interface Props {
    user: {
        username: string,
        pictureUrl: string,
    }
}

function InlineUser({ user }: Props) {
    return (
        <Link to={`/user/${user.username}`} className="text-lg">
            <img src={user.pictureUrl} draggable={false} className="size-10 rounded-full inline mr-3" />
            {user.username}
        </Link>
    );
}

export default InlineUser;