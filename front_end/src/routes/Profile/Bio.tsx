import { useState } from "react";
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome'
import { faPencil } from '@fortawesome/free-solid-svg-icons'
import { fetchApi } from "../../utils";
import TextArea from "../../components/TextArea";


function Bio({ value: defaultValue }: { value: string }) {
    const [editing, setEditing] = useState(false);

    const [contents, setContents] = useState(defaultValue);
    const maxLength = 100;

    const inputFilter = (input: string) => input.replace(/\n/g, "");

    async function saveChanges(contents: string) {
        await fetchApi("/profile/bio", {
            method: "PATCH",
            headers: {
                "Content-Type": "application/json",
            },
            body: JSON.stringify({
                bio: contents,
            })
        });
        
        setContents(contents);
        setEditing(false);
    }

    return (
        <div className="py-5">
            <div className="flex flex-row items-center">
                <h4 className="text-xl pr-1">Bio</h4>
                {editing || <FontAwesomeIcon onClick={() => setEditing(true)} icon={faPencil} />}
            </div>
            {editing ?
                <TextArea className="bg-white outline" submitText="Save Changes" onSubmit={saveChanges} value={contents} {...{maxLength, inputFilter}} /> :
                <p>{contents || "You don't currently have a bio."}</p>
            }
        </div>
    )
}

export default Bio;