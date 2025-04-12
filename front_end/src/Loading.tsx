import { FontAwesomeIcon } from '@fortawesome/react-fontawesome'
import { faCircleNotch } from '@fortawesome/free-solid-svg-icons'

const Loading = () => (
    <FontAwesomeIcon className="absolute top-[30%] left-[50%]  animate-spin" icon={faCircleNotch} size="4x"/>
);

export default Loading;