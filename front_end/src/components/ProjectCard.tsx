import { useNavigate } from "react-router";
import { ProjectInfo } from "../types";

function Tag({ contents }: { contents: string }) {
    return (
        // TODO: add triangle tag shape (wip https://jsfiddle.net/zcdLjmsf/)
        <div className="inline bg-light-gray text-black px-1 py-0.5 mr-1">{contents}</div>
    )
}


function ProjectCard({ project }: { project: ProjectInfo }) {
    const navigate = useNavigate();
    const goToProject = () => navigate(`/project/${project.username}/${project.repoName}`)

    return (
        <div onClick={goToProject} className="rounded-lg bg-blue-gray font-light p-5 w-96 h-72">
            <h4 className="text-3xl">{project.title}</h4>
            <div className="my-2">
                {project.tags.map((tag) => <Tag contents={tag} />)}
            </div>
            <p>{project.readme}</p>
        </div>
    );
}

export default ProjectCard;