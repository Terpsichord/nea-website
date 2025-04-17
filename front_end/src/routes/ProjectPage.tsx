import { faEllipsisV, faHeart as faHeartSolid } from "@fortawesome/free-solid-svg-icons";
import { faHeart as faHeartRegular } from "@fortawesome/free-regular-svg-icons";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { useEffect, useRef, useState } from "react";
import ContextMenu from "../components/ContextMenu";
import { Link, useParams } from "react-router";
import { fetchApi, formatDate, useApi } from "../utils";
import { Project } from "../types";
import Loading from "../components/Loading";
import DOMPurify from "dompurify";
import { marked } from "marked";
import '../markdown.scss';
import { useAuth } from "../auth";

function ProjectPage() {
    const params = useParams();
    const [project, _error] = useApi<Project>(`/project/${params.username}/${params.id}`)

    const { isAuth } = useAuth()
    const [likedInitial] = useApi<boolean>(isAuth ? `/project/${params.username}/${params.id}/liked`: null, { deps: [isAuth] }) ?? [undefined];

    const [liked, setLiked] = useState(false);
    const [likeCount, setLikeCount] = useState(0);

    useEffect(() => {
        if (likedInitial !== undefined) {
            setLiked(likedInitial);
        }
    }, [likedInitial]);

    useEffect(() => {
        if (project) {
            setLikeCount(project.likeCount);
        }
    }, [project]);

    const [showMenu, setShowMenu] = useState(false);
    const menuParent = useRef<HTMLDivElement | null>(null);

    if (project === undefined) {
        return <Loading />;
    }

    const menuItems = [
        <a href={project.githubUrl}>View files on Github</a>,
        <a href="editor_url">View in editor</a>
    ];

    const onLikeClick = () => {
        if (!isAuth) return;

        if (liked) {
            setLikeCount(likeCount - 1);
            fetchApi(`/project/${params.username}/${params.id}/unlike`, { method: "POST" });
        } else {
            setLikeCount(likeCount + 1);
            fetchApi(`/project/${params.username}/${params.id}/like`, { method: "POST" });
        }

        setLiked(!liked);
    }

    const readmeHtml = DOMPurify.sanitize(marked.parse(project.readme, { async: false }));
    const uploadDate = formatDate(new Date(project.uploadTime));

    // TODO: show tags

    return (
        <div className="px-24">
            <h2 className="text-4xl font-medium mb-3">{project.title}</h2>
            <div className="flex items-center mb-7">
                <img src={project.pictureUrl} draggable={false} className="size-10 rounded-full" />
                <Link to={`/user/${project.username}`} className="pl-3 text-lg">{project.username}</Link>
                <div ref={menuParent} className="ml-auto" onClick={() => setShowMenu(true)}>
                    <FontAwesomeIcon icon={faEllipsisV} className="px-3" size="lg" />
                    {showMenu &&
                        <ContextMenu items={menuItems} parent={menuParent} setShow={setShowMenu} />
                    }
                </div>
            </div>
            <div className="markdown bg-blue-gray rounded-2xl p-8 mb-5" dangerouslySetInnerHTML={{ __html: readmeHtml }} />
            <div className="flex text-gray">
                <span>Uploaded {uploadDate}</span>
                <span className="ml-auto mr-1">
                    <FontAwesomeIcon icon={liked ? faHeartSolid : faHeartRegular} onClick={onLikeClick} className="mr-1" />
                    {likeCount} like{likeCount === 1 ? "" : "s"}
                </span>
            </div>
        </div>
    )
}

export default ProjectPage;