import { useNavigate } from "react-router";
import { useAuth } from "../../auth";
import { useEffect } from "react";
import Bio from "./Bio";
import Loading from "../../components/Loading";
import { formatDate, useApi } from "../../utils";
import { User } from "../../types";
import AccentBox from "../../components/AccentBox";
import Button from "../../components/Button";
import Followers from "./Followers";

function Profile() {
    const { isAuth, signOut, signedOut } = useAuth();
    const navigate = useNavigate();

    useEffect(() => {
        if (!isAuth && !signedOut) {
            navigate("/signin");
        }
    }, [isAuth]);

    const [user] = useApi<User>(isAuth ? "/profile" : null) ?? [undefined];
    const [followers] = useApi<User[]>(isAuth ? "/follow/followers" : null) ?? [undefined];

    if (user === undefined) {
        return <Loading />;
    }

    const joinDate = formatDate(new Date(user.joinDate));
    return (
        <div className="margin mx-auto flex flex-col justify-center items-center">
            <AccentBox size="lg">
                <h3 className="text-2xl font-medium">My Profile</h3>
                <div className="flex items-center py-5">
                    <img src={user.pictureUrl} draggable={false} className="size-26 outline-3 outline-gray rounded-full" />
                    <h2 className="pl-10 font-medium text-3xl">{user.username}</h2>
                </div>
                <span>Joined {joinDate}</span>
                <Bio value={user.bio} />
                <div className="space-y-2">
                    <Button onClick={() => navigate(`/user/${user.username}`)}>Go to user page</Button>
                    <Button onClick={signOut} color="red">Sign-out</Button>
                </div>
            </AccentBox>
            {followers && followers.length > 0 && <Followers followers={followers} /> }
        </div>
    );
}

export default Profile;
