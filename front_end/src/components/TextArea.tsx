import { ChangeEvent, useState } from "react";

interface TextAreaProps {
    value?: string,
    maxLength: number,
    inputFilter?: (input: string) => string,
    submitText: string,
    onSubmit: (contents: string) => void,
    className?: string,
    submitClass?: string
}

function TextArea({ value: defaultValue = "", maxLength, inputFilter, submitText, onSubmit, className = "", submitClass = "" }: TextAreaProps) {
    const [contents, setContents] = useState(defaultValue);

    function onChange(event: ChangeEvent<HTMLTextAreaElement>) {
        let value = event.target.value.slice(0, maxLength);

        if (inputFilter) {
            value = inputFilter(value);
        }

        setContents(value);
    }

    function onClick() {
        onSubmit(contents);
    }

    return (
        <div className="flex flex-col">
            <div className="relative">
                <textarea className={`${className} rounded-lg resize-none w-full h-24 p-1 overflow-hidden`} value={contents} onChange={onChange} />
                <div className="font-light absolute right-0 bottom-0 pr-1 pb-1">{contents.length}/{maxLength}</div>
            </div>
            <button className={`${submitClass} self-end`} onClick={onClick}>{submitText}</button>
        </div>
    )
}

export default TextArea;