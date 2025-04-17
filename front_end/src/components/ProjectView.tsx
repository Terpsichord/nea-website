import ProjectCard from "./ProjectCard";
import { ProjectInfo } from "../types";
import { useApi } from "../utils";
import Loading from "./Loading";

function ProjectView({ username, dashboard, className }: { username?: string, dashboard?: boolean, className: string }) {
    let projects, error;
    if (dashboard) {
        [projects, error] = useApi<ProjectInfo[]>(`/profile/projects`);
    }
    else if (username === undefined) {
        [projects, error] = useApi<ProjectInfo[]>("/projects");
    } else {
        [projects, error] = useApi<ProjectInfo[]>(`/user/${username}/projects`);
    }

    if (projects === undefined) {
        return <Loading />
    }

    if (error) {
        return <span className="text-red-600 italic">Failed to load projects</span>;
    }

    return (
        <div className="flex justify-center">
            <div className={`${className} grid`}>
                {projects.map((project) => (
                    <ProjectCard project={project} />
                ))}
            </div>
        </div>
    )
}

export default ProjectView;