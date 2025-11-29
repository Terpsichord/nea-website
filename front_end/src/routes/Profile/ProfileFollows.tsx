import { useNavigate } from "react-router";
import { useAuth } from "../../auth";
import InlineUserView from "../../components/InlineUser";
import { InlineUser } from "../../types";
import { useEffect } from "react";
import Loading from "../../components/Loading";

function ProfileFollows({ followers, followings }: { followers?: InlineUser[], followings?: InlineUser[] }) {
    const { isAuth, signedOut } = useAuth();
    const navigate = useNavigate();

    useEffect(() => {
        if (!isAuth && !signedOut) {
            navigate("/signin");
        }
    }, [isAuth]);

    if (followers === undefined || followings === undefined) {
        return <Loading />;
    }

    return (
        <>
            <div className="w-full px-16">
                <hr className="h-px w-full bg-white border-0" />
            </div>
            <div className="self-left w-full pt-12 px-16">
                <div className="w-1/2 float-left">
                    <h2 className="font-medium text-3xl pb-8">Your followers</h2>
                    <ul>
                        {followers.map(follower => <li><InlineUserView user={follower} /></li>)}
                    </ul>
                </div>
                <div className="w-1/2 float-right">
                    <h2 className="font-medium text-3xl pb-8">Users you follow</h2>
                    <ul>
                        {followings.map(following => <li><InlineUserView user={following} /></li>)}
                    </ul>
                </div>
            </div>
        </>
    )
}

export default ProfileFollows;