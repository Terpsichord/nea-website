import { useLocation, useNavigate } from "react-router";
import { useAuth } from "../../auth";
import { useEffect } from "react";
import { useApi } from "../../utils";
import { InlineUser, User } from "../../types";
import ProfileFollows from "./ProfileFollows";
import ProfileHeader from "./ProfileHeader";
import ProfileInfo from "./ProfileInfo";

function Profile() {
    const { isAuth, signedOut } = useAuth();
    const navigate = useNavigate();
    const location = useLocation();

    const isFollows = location.hash === "#follows";

    useEffect(() => {
        if (!isAuth && !signedOut) {
            navigate("/signin");
        }
    }, [isAuth]);

    const [user] = useApi<User>(isAuth ? "/profile" : null) ?? [undefined];
    const [followers] = useApi<InlineUser[]>(isAuth ? "/follow/followers" : null) ?? [undefined];
    const [followings] = useApi<InlineUser[]>(isAuth ? "/follow" : null) ?? [undefined];

    return (
        <div className="margin mx-auto flex flex-col justify-center items-center">
            <ProfileHeader follows={isFollows} />
            { isFollows ? <ProfileFollows followers={followers} followings={followings}/> : <ProfileInfo user={user} />}
        </div>
    );
}

export default Profile;
