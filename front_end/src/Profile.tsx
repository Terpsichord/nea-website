import { useNavigate } from "react-router";
import { useAuth } from "./AuthProvider";
import { useEffect } from "react";
import Bio from "./Bio";
import Loading from "./Loading";
import { formatDate, useQuery } from "./utils";
import { User } from "./types";


function Profile() {
    const auth = useAuth();
    const navigate = useNavigate();

    useEffect(() => {
        if (!auth.isAuth) {
            navigate("/signin");
        }
    }, [auth]);

    const [user, loading] = useQuery<User>("/api/profile");

    if (loading) {
        return <Loading />
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
                    <button className="text-white bg-red-400 outline-2 outline-red-700 rounded-xl" onClick={auth.signOut}>Sign-out</button>
                </div>
            </div>
    )
}

export default Profile;
