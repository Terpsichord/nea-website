import { FontAwesomeIcon } from '@fortawesome/react-fontawesome'
import { faCircleNotch } from '@fortawesome/free-solid-svg-icons'

const Loading = () => (
    <FontAwesomeIcon
        icon={faCircleNotch}
        size="4x"
        className="absolute left-1/2 top-[30%] -translate-x-1/2 animate-spin"
    />
);

export default Loading;