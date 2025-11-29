import { faXmark } from "@fortawesome/free-solid-svg-icons";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";

type TagProps = {
    contents: string,
} & ({
    index: number,
    onRemove: (i: number) => void,
} | {
    index?: undefined,
    onRemove?: undefined,
});

function Tag({ contents, index, onRemove }: TagProps) {
    return (
        <div key={index} className="inline bg-light-gray text-black px-1 py-0.5 space-x-1">
            <span>{contents}</span>
            {onRemove && <FontAwesomeIcon icon={faXmark} className="cursor-pointer" onClick={() => onRemove(index)} />}
        </div>
    )
}

export default Tag;