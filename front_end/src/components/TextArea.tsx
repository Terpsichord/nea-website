import { ChangeEvent, KeyboardEvent, ReactElement, Ref, useState } from "react";

interface TextAreaProps {
    ref?: Ref<HTMLTextAreaElement>,
    value?: string,
    subtext?: ReactElement,
    maxLength: number,
    inputFilter?: (input: string) => string,
    submitText: string,
    onSubmit: (contents: string) => void,
    className?: string,
    submitClass?: string
}

function TextArea({ ref, value: defaultValue = "", subtext, maxLength, inputFilter, submitText, onSubmit, className = "", submitClass = "" }: TextAreaProps) {
    const [contents, setContents] = useState(defaultValue);

    function onChange(event: ChangeEvent<HTMLTextAreaElement>) {
        let value = event.target.value.slice(0, maxLength);

        if (inputFilter) {
            value = inputFilter(value);
        }

        setContents(value);
    }

    const onClick = () => onSubmit(contents);

    function onKeyDown(event: KeyboardEvent<HTMLTextAreaElement>) {
        if (event.ctrlKey && event.key == "Enter") {
            onSubmit(contents);
        }
    }

    return (
        <div className="flex flex-col">
            <div className="relative">
                <textarea ref={ref} className={`${className} rounded-lg resize-none w-full h-24 p-1 overflow-hidden`} value={contents} onChange={onChange} onKeyDown={onKeyDown}/>
                <div className="font-light absolute right-0 bottom-0 pr-1 pb-1">{contents.length}/{maxLength}</div>
            </div>
            <div className="flex justify-between">
                {subtext}
                <button className={`${submitClass} ml-auto self-start`} onClick={onClick}>{submitText}</button>
            </div>
        </div>
    )
}

export default TextArea;