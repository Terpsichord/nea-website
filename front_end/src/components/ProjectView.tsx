import ProjectCard from "./ProjectCard";
import { ProjectInfo } from "../types";
import { ApiError } from "../utils";
import Loading from "./Loading";

function ProjectView({ projects, error, horizontal }: { projects: ProjectInfo[] | undefined, error?: ApiError, horizontal?: boolean }) {
    if (projects === undefined) {
        return <Loading />
    }

    if (error) {
        return <span className="text-red-600 italic">Failed to load projects</span>;
    }

    const projectCards = projects.map(project => <ProjectCard project={project} />);

    return horizontal ? <div className="grid grid-flow-col overflow-x-auto gap-4 p-4">
        {projectCards}
    </div> :
    <div className="flex justify-center">
        <div className="grid lg:grid-cols-2 grid-cols-1 gap-x-20 gap-y-14">
            {projectCards}
        </div>
    </div>;
}

export default ProjectView;