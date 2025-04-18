import { useNavigate } from "react-router";
import { useAuth } from "../auth";
import { useEffect } from "react";
import Bio from "../components/Bio";
import Loading from "../components/Loading";
import { formatDate, useApi } from "../utils";
import { User } from "../types";

function Profile() {
    const { isAuth, signOut, signedOut } = useAuth();
    const navigate = useNavigate();

    useEffect(() => {
        if (!isAuth && !signedOut) {
            navigate("/signin");
        }
    }, [isAuth]);

    const [user] = useApi<User>(isAuth ? "/profile" : null) ?? [undefined];
    const [followers] = useApi<User[]>(isAuth ? "/followers" : null) ?? [undefined];

    if (user === undefined) {
        return <Loading />;
    }

    const joinDate = formatDate(new Date(user.joinDate));
    return (
        <div className="flex justify-center items-center">
            <div className="flex flex-col items-start justify-center bg-light-gray text-black rounded-4xl w-300 py-8 px-16">
                <h3 className="text-2xl font-medium">My Profile</h3>
                <div className="flex items-center py-5">
                    <img src={user.pictureUrl} draggable={false} className="size-26 outline-3 outline-gray rounded-full" />
                    <h2 className="pl-10 font-medium text-3xl">{user.username}</h2>
                </div>
                <span>Joined {joinDate}</span>
                <Bio value={user.bio} />
                <div className="space-y-2">
                    <button className="block bg-light-gray outline-2 rounded-xl px-2" onClick={() => navigate(`/user/${user.username}`)}>Go to user page</button>
                    <button className="block text-white bg-red-400 outline-2 outline-red-700 rounded-xl px-2" onClick={signOut}>Sign-out</button>
                </div>
            </div>
            {followers === undefined ?
                <Loading /> :
                <ul>
                    {followers.map((follower) => <li>{follower.username} ({follower.bio})</li>)}
                </ul>
            }
        </div>
    );
}

export default Profile;
