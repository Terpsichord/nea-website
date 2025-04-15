import { useParams } from "react-router";
import { formatDate, useQuery } from "../utils";
import Loading from "../components/Loading";
import { User } from "../types";
import ProjectView from "../components/ProjectView";
import { useState } from "react";
import Follow from "../components/Follow";
import { useAuth } from "../auth";

function UserPage() {
    const params = useParams();

    const { isAuth } = useAuth();
    const [user, error] = useQuery<User>("/api/user/" + params.username);

    const [showFollow, setShowFollow] = useState(false);

    if (user === undefined) {
        return <Loading />;
    }

    if (error) {
        return "Failed to load user profile";
    }

    const joinDate = formatDate(new Date(user.joinDate));
    return (
        <div className="pl-24 min-h-screen">
            <div className="flex items-center py-5">
                <img src={user.pictureUrl} draggable={false} className="size-32 rounded-full mb-4" />
                <div className={`pl-6 ${showFollow ? "pt-3" : "pb-3"}`}>
                    <h2 className="font-medium text-4xl">{user.username}</h2>
                    {isAuth &&
                        <Follow username={user.username} setShow={setShowFollow} />
                    }
                </div>
            </div>
            <p className="text-2xl">Joined {joinDate}</p>
            <p className="pl-4 my-6 text-gray text-2xl">{user.bio}</p>
            <h2 className="text-4xl">Projects</h2>
            <div className="mt-5">
                <ProjectView username={user.username} className="lg:grid-cols-2 grid-cols-1 gap-x-20 gap-y-14" />
            </div>
        </div>
    )
}

export default UserPage;