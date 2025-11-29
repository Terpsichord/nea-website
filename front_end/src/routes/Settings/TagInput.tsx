import React, { Dispatch, useState } from "react";
import Tag from "../../components/Tag";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faRotateRight } from "@fortawesome/free-solid-svg-icons";

function TagInput({ tags, setTags, initialTags, className }: { tags: string[], setTags: Dispatch<string[]>, initialTags: string[], className?: string }) {
    function removeTag(i: number) {
        setTags(tags.filter((_, index) => index !== i));
    }

    function handleKeyDown(e: React.KeyboardEvent<HTMLInputElement>) {
        if (e.key === "Enter" && input.trim()) {
            e.preventDefault();
            setTags([...tags, input]);
            setInput("");
        }
        if (e.key === "Backspace") {
            if (input === "" && tags.length > 0) {
                removeTag(tags.length - 1);
            }
        }
    }

    function reset() {
        setTags(initialTags);
        setInput("");
    }

    const [input, setInput] = useState("");

    return (
        <div className={`flex relative items-center space-x-1 ${className}`}>
            {tags.map((tag, index) => <Tag contents={tag} index={index} onRemove={removeTag} />)}
            <input
                value={input}
                onChange={e => setInput(e.target.value)}
                onKeyDown={handleKeyDown}
                className="inline grow h-full outline-none"
            />
            <FontAwesomeIcon icon={faRotateRight} onClick={reset} className="absolute right-0 pr-2 top-1/2 transform -translate-y-1/2 cursor-pointer" />
        </div>
    )
}

export default TagInput;