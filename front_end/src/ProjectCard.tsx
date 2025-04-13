import { ProjectInfo } from "./types";

function Tag({ contents }: { contents: string }) {
    return (
        <div className="inline bg-white text-black px-1 py-0.5 mr-1">{contents}</div>
    )
}

function ProjectCard({ project }: { project: ProjectInfo }) {
    return (
        <div className="rounded-lg bg-blue-gray font-light p-5 w-96 h-72">
            <h4 className="text-3xl">{project.title}</h4>
            <div className="my-2">
                {project.tags.map((tag) => <Tag contents={tag} />)}
            </div>
            <p>{project.readme}</p>
        </div>
    );
}

export default ProjectCard;