import { useParams } from "react-router";
import { formatDate, useQuery } from "./utils";
import Loading from "./Loading";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faPlus } from "@fortawesome/free-solid-svg-icons";
import { User, ProjectInfo } from "./types";
import ProjectView from "./ProjectView";

function UserPage() {
    const params = useParams();

    const [user, loadingUser, userError] = useQuery<User>("/api/user/" + params.username);
    // const [projects, loadingProjects, projectsError] = useQuery<ProjectInfo[]>(`/api/user/${params.username}/projects`)
    const projects: ProjectInfo[] = Array(5).fill(
        {
            title: "my-ide",
            tags: ["compiler", "text-editor"],
            readme: "This is simple small text editor project to practice programming an IDE that can be used for writing, as well as running code."
        }
    );

    if (loadingUser) {
        return <Loading />;
    }

    if (userError) {
        return "Failed to load user profile";
    }

    const joinDate = formatDate(new Date(user.joinDate));
    return (
        <div className="pl-24 min-h-screen">
            <div className="flex items-center py-5">
                <img src={user.pictureUrl} draggable={false} className="size-32 rounded-full mb-4" />
                <div className="pl-6 pt-3">
                    <h2 className="font-medium text-4xl">{user.username}</h2>
                    <button className="bg-white text-black mt-2 px-2.5 py-0.5 rounded-xl">
                        <FontAwesomeIcon icon={faPlus} size="xl" className="pr-1.5 pb-0.5" />
                        <span className="text-2xl font-medium">Follow</span>
                    </button>
                </div>
            </div>
            <p className="text-2xl">Joined {joinDate}</p>
            <p className="pl-4 my-6 text-gray text-2xl">{user.bio}</p>
            <h2 className="text-4xl">Projects</h2>
            <div className="mt-5">
            {/* {loadingProjects ? <Loading /> : (projectsError ? <span className="text-red-600 italic">Failed to load projects</span> :  */}
            <ProjectView projects={projects} className="lg:grid-cols-2 grid-cols-1 gap-x-20 gap-y-14" />
            {/* )} */}
            </div>
        </div>
    )
}

export default UserPage;