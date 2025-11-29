import { faXmark } from "@fortawesome/free-solid-svg-icons";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { Dispatch } from "react";
import { fetchApi } from "../../utils";
import { useAuth } from "../../auth";

function DeleteModal({ setShowModal }: { setShowModal: Dispatch<boolean> }) {
    const { signOut } = useAuth();

    async function deleteAccount() {
        await fetchApi("/profile/delete", { method: "DELETE" });
        signOut();
    }

    return (
        <div className="flex-col fixed top-0 left-0 w-full h-full bg-black/45">
            <button onClick={() => setShowModal(false)} className="absolute right-16 top-16 text-2xl">
                <FontAwesomeIcon icon={faXmark} className="cursor-pointer" />
            </button>
            <div className="flex flex-col py-8 px-10 rounded-3xl m-10 bg-blue-gray text-white">
                <h2 className="font-medium text-3xl pb-6">Confirm account deletion</h2>
                <div className="outline outline-red-800 bg-red-200 text-red-800 p-3 rounded-xl">
                    <span className="font-medium">Warning: </span>
                    This will permananetly delete your account, and all associated projects. (You will still be able to access them on GitHub)
                </div>
                <button onClick={deleteAccount} className="cursor-pointer outline-3 outline-red-800 bg-red-500 text-2xl font-bold px-5 py-2.5 mt-24 rounded-xl">Delete Account</button>
            </div>
        </div>
    )
}

export default DeleteModal;