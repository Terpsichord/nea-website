import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import ProjectView from "../components/ProjectView";
import { faPlus } from "@fortawesome/free-solid-svg-icons";

function Dashboard() {
    return (
        <div className="container mx-auto">
            <div className="flex justify-center items-center">
                <button className="outline-2 rounded-md text-2xl p-5 my-32">
                    <FontAwesomeIcon className="mr-2" icon={faPlus} />
                    Create new project
                </button>
            </div>
            <h2 className="text-4xl mb-5">Your projects</h2>
            <ProjectView dashboard className="lg:grid-cols-2 grid-cols-1 gap-x-20 gap-y-14" />
        </div>
    );
}

export default Dashboard;