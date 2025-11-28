import { faEllipsisV, faGlobe, faHeart as faHeartSolid, faLock } from "@fortawesome/free-solid-svg-icons";
import { faHeart as faHeartRegular } from "@fortawesome/free-regular-svg-icons";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { useEffect, useRef, useState } from "react";
import ContextMenu from "../../components/ContextMenu";
import { useParams } from "react-router";
import { fetchApi, formatDate, useApi } from "../../utils";
import { Project } from "../../types";
import Loading from "../../components/Loading";
import DOMPurify from "dompurify";
import { marked } from "marked";
import './markdown.scss';
import { useAuth } from "../../auth";
import InlineUserView from "../../components/InlineUser";
import Comments from "./Comments";
import Tag from "../../components/Tag";

function ProjectPage() {
    const params = useParams();
    const [project, projectError] = useApi<Project>(`/project/${params.username}/${params.id}`)

    const { isAuth } = useAuth()
    const [likedInitial] = useApi<boolean>(isAuth ? `/project/${params.username}/${params.id}/liked` : null, { deps: [isAuth] }) ?? [undefined];

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


    if (projectError?.status === 404) {
        return (
            <div className="flex justify-center">
                <p className="text-4xl font-medium my-[30vh]">404 - Project not found</p>
            </div>
        );
    }

    if (project === undefined) {
        return <Loading />;
    }

    let menuItems = [
        <a href={project.githubUrl}>View files on Github</a>,
    ];

    if (project.owned) {
        menuItems.push(<a href={`/editor/${params.username}/${params.id}`}>View in editor</a>);
    }

    const remixProject = async () => {
        const response = await fetchApi(`/project/${params.username}/${params.id}/remix`, { method: "POST" });

        if (response.ok) {
            const { username, repo_name } = await response.json();
            window.location.href = `/editor/${username}/${repo_name}`
        } else {
            // TODO: error handling
        }
    };

    if (isAuth && !project.owned) {
        menuItems.push(<a onClick={remixProject} href={`/project/${params.username}/${params.id}/remix`}>Remix</a>);
    }


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
                <InlineUserView user={project} />
                <div className="bg-blue-gray rounded-2xl ml-3 px-2.5 py-1">{project.public ?
                    <><FontAwesomeIcon icon={faGlobe} size="sm" className="mr-1.5" />Public</> :
                    <><FontAwesomeIcon icon={faLock} size="sm" className="mr-1.5" />Private</>
                }</div>

                <div ref={menuParent} className="ml-auto" onClick={() => setShowMenu(true)}>
                    <FontAwesomeIcon icon={faEllipsisV} className="px-3" size="lg" />
                    {showMenu &&
                        <ContextMenu items={menuItems} parent={menuParent} setShow={setShowMenu} />
                    }
                </div>
            </div>

            <div className="markdown bg-blue-gray rounded-2xl px-8 pb-4 pt-4 mb-3">
                <div dangerouslySetInnerHTML={{ __html: readmeHtml }} />
                <div className="mt-5 space-x-2">{project.tags.map(tag => <Tag contents={tag} />)}</div>
            </div>

            <div className="flex text-gray">
                <span>Uploaded {uploadDate}</span>
                <span className="ml-auto mr-1">
                    <FontAwesomeIcon icon={liked ? faHeartSolid : faHeartRegular} onClick={onLikeClick} className="mr-1" />
                    {likeCount} like{likeCount === 1 ? "" : "s"}
                </span>
            </div>
            <Comments project={{ username: params.username!, id: params.id! }} />
        </div>
    )
}

export default ProjectPage;