import { useNavigate } from "react-router";
import { useAuth } from "../../auth";
import AccentBox from "../../components/AccentBox";
import Button from "../../components/Button";
import Bio from "./Bio";
import { formatDate } from "../../utils";
import Loading from "../../components/Loading";
import { User } from "../../types";
import { useState } from "react";
import DeleteModal from "./DeleteModal";

function ProfileInfo({ user }: { user?: User }) {
    const { signOut } = useAuth();
    const navigate = useNavigate();

    const [showModal, setShowModal] = useState(false);

    if (user === undefined) {
        return <Loading />;
    }

    const joinDate = formatDate(new Date(user.joinDate));
    return (
        <>
            <AccentBox size="lg">
                <div className="flex items-center py-5">
                    <img src={user.pictureUrl} draggable={false} className="size-26 outline-3 outline-gray rounded-full" />
                    <h2 className="pl-10 font-medium text-3xl">{user.username}</h2>
                </div>
                <span>Joined {joinDate}</span>
                <Bio value={user.bio} />
                <div className="space-y-2">
                    <Button onClick={() => navigate(`/user/${user.username}`)}>Go to user page</Button>
                    <Button onClick={signOut} color="red">Sign-out</Button>
                    <div className="mt-5">
                        <Button onClick={() => setShowModal(true)} color="red">Delete account</Button>
                    </div>
                </div>
            </AccentBox>
            {showModal && <DeleteModal setShowModal={setShowModal} />}
        </>
    );
}

export default ProfileInfo;