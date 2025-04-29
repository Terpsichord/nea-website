import InlineUserView from "../../components/InlineUser";
import { User } from "../../types";

function Followers({followers}: {followers: User[]}) {
    return (
        <div className="self-left w-full pt-12 px-16">
            <h2 className="font-medium text-3xl pb-8">Followers</h2>
            <ul>
                {followers.map((follower) => <li><InlineUserView user={follower}/></li>)}
            </ul>
        </div>
    )
}

export default Followers;