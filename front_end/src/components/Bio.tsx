import { ChangeEvent, useState } from "react";
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome'
import { faPencil } from '@fortawesome/free-solid-svg-icons'
import { fetchApi } from "../utils";


function Bio({ value }: { value: string }) {
    const [editing, setEditing] = useState(false);

    const [contents, setContents] = useState(value);
    const maxLength = 100;

    function onChange(event: ChangeEvent<HTMLTextAreaElement>) {
        const value = event.target.value.slice(0, maxLength).replace(/\n/g, "");
        setContents(value);
    }

    async function saveChanges() {
        await fetchApi("/profile/bio", {
            method: "PATCH",
            headers: {
                "Content-Type": "application/json",
            },
            body: JSON.stringify({
                bio: contents,
            })
        })
        
        setEditing(false);
    }

    return (
        <div className="py-5">
            <div className="flex flex-row items-center">
                <h4 className="text-xl pr-1">Bio</h4>
                {editing || <FontAwesomeIcon onClick={() => setEditing(true)} icon={faPencil} />}
            </div>
            {editing ?
                <div className="flex flex-col">
                    <div className="relative">
                        <textarea className="bg-white rounded-lg outline resize-none w-100 h-24 p-1" value={contents} onChange={onChange} />
                        <div className="font-light absolute right-0 bottom-0 pr-1 pb-1">{contents.length}/{maxLength}</div>
                    </div>
                    <button className="self-end" onClick={saveChanges}>Save changes</button>
                </div> :
                <p>{contents || "You don't currently have a bio."}</p>
            }
        </div>
    )
}

export default Bio;