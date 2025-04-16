import { faEllipsisV } from "@fortawesome/free-solid-svg-icons";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { useEffect, useRef, useState } from "react";
import ContextMenu from "../components/ContextMenu";
import { useParams } from "react-router";
import { useQuery } from "../utils";
import { Project } from "../types";
import Loading from "../components/Loading";

function ProjectPage() {
    const params = useParams();
    const [project, _error] = useQuery<Project>(`/api/project/${params.username}/${params.id}`)

    const [showMenu, setShowMenu] = useState(false);

    const menuParent = useRef<HTMLDivElement | null>(null);


    useEffect(() => console.log(project?.githubUrl), [project]);

    if (project === undefined) {
        return <Loading />;
    }

    const menuItems = [
        <a href={project.githubUrl}>View files on Github</a>,
        <a href="editor_url">View in editor</a>
    ];

    return (
        <div className="px-24">
            <h2 className="text-4xl font-medium mb-3">{project.title}</h2>
            <div className="flex items-center mb-7">
                <img src={project.pictureUrl} draggable={false} className="size-10 rounded-full" />
                <span className="pl-3 text-lg">{project.username}</span>
                <div ref={menuParent} className="ml-auto" onClick={() => setShowMenu(true)}>
                    <FontAwesomeIcon icon={faEllipsisV} className="px-3" size="lg" />
                    {showMenu &&
                        <ContextMenu items={menuItems} parent={menuParent} setShow={setShowMenu} />
                    }
                </div>
            </div>
            <div className="bg-blue-gray rounded-2xl p-8">{project.readme}</div>
        </div>
    )
}

export default ProjectPage;