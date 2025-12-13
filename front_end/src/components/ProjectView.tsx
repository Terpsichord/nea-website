import ProjectCard from "./ProjectCard";
import { ProjectInfo } from "../types";
import { ApiError } from "../utils";
import Loading from "./Loading";

function ProjectView({ projects, error, className }: { projects: ProjectInfo[] | undefined, error?: ApiError, className: string }) {
    if (projects === undefined) {
        return <Loading />
    }

    if (error) {
        return <span className="text-red-600 italic">Failed to load projects</span>;
    }

    return (
        <div className="flex justify-center">
            <div className={`${className} grid`}>
                {projects.map(project => <ProjectCard project={project} />)}
            </div>
        </div>
    )
}

export default ProjectView;