import ProjectCard from "./ProjectCard";
import { ProjectInfo } from "../types";
import { useApi } from "../utils";
import Loading from "./Loading";

function ProjectView({ username, className }: { username: string, className: string }) {
    const [projects, error] = useApi<ProjectInfo[]>(`/user/${username}/projects`)

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