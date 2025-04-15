import { Dispatch, useEffect, useState } from "react";
import { User } from "../types";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faCheck, faPlus } from "@fortawesome/free-solid-svg-icons";
import { useQuery } from "../utils";

function Follow({ username, setShow }: { username: string, setShow: Dispatch<boolean> }) {
    const [isFollowed, setIsFollowed] = useState(false);

    const [signedInUser] = useQuery<User>("/api/profile");
    const [isFollowedInitial] = useQuery<boolean>(`/api/follow/${username}`);

    useEffect(() => {
        if (isFollowedInitial !== undefined) {
            setIsFollowed(isFollowedInitial);
        }
    }, [isFollowedInitial]);

    const canFollow = !(signedInUser === undefined || isFollowedInitial === undefined || isFollowed || username === signedInUser.username);
    useEffect(() => setShow(isFollowed || canFollow), [isFollowed, canFollow]);

    const follow = () => {
        fetch(`/api/follow/${username}`, { method: "POST" })
        setIsFollowed(true);
    };
    const unfollow = () => {
        fetch(`/api/follow/${username}/unfollow`, { method: "POST" });
        setIsFollowed(false);
    };

    if (isFollowed) {
        return (
            <button onClick={unfollow} className="bg-black mt-2 px-2.5 py-0.5 rounded-xl">
                <FontAwesomeIcon icon={faCheck} size="xl" className="pr-1.5 pb-0.5" />
                <span className="text-2xl font-medium">Unfollow</span>
            </button>
        );
    } else if (canFollow) {
        return (
            <button onClick={follow} className="bg-white text-black mt-2 px-2.5 py-0.5 rounded-xl">
                <FontAwesomeIcon icon={faPlus} size="xl" className="pr-1.5 pb-0.5" />
                <span className="text-2xl font-medium">Follow</span>
            </button>
        );
    } else {
        return <></>;
    }
}

export default Follow;