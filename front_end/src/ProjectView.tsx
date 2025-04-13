import ProjectCard from "./ProjectCard";
import { ProjectInfo } from "./types";

function ProjectView({ projects, className }: { projects: ProjectInfo[], className: string }) {
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