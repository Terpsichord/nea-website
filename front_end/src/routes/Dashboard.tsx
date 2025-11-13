import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import ProjectView from "../components/ProjectView";
import { faPlus, faXmark } from "@fortawesome/free-solid-svg-icons";
import { FormEvent, useState } from "react";
import { fetchApi } from "../utils";
import { useNavigate } from "react-router";

function Dashboard() {
    const [showModal, setShowModal] = useState(false);

    const navigate = useNavigate();

    async function createProject(e: FormEvent<HTMLFormElement>) {
        e.preventDefault();

        const projectData = new FormData(e.target as HTMLFormElement);
        const title = projectData.get("title") as string;
        const visibility = projectData.get("title") as string;

        const response = await fetchApi("/project/new", {
            method: "POST",
            headers: {
                "Content-Type": "application/json",
            },
            body: JSON.stringify({ title, lang: "// TODO", private: visibility == "private" })
        });

        console.log({ response });
        if (response.ok) {
            const { username, repo_name } = await response.json();
            window.location.href = `/editor/${username}/${repo_name}`
        } else {
            // TODO: error handling
        }
    };

    return <>
        <div className="container mx-auto">
            <div className="flex justify-center items-center">
                <button onClick={() => setShowModal(true)} className="outline-2 rounded-md text-2xl p-5 my-32 hover:bg-white hover:text-black">
                    <FontAwesomeIcon className="mr-2" icon={faPlus} />
                    Create new project
                </button>
            </div>
            <h2 className="text-4xl mb-5">Your projects</h2>
            <ProjectView dashboard className="lg:grid-cols-2 grid-cols-1 gap-x-20 gap-y-14" />
        </div>
        {
            showModal &&
            <div className="fixed top-0 left-0 w-full h-full bg-black/45">
                <form onSubmit={createProject} className="flex flex-col py-8 px-10 rounded-3xl m-10 bg-blue-gray">
                    <button onClick={() => setShowModal(false)} className="absolute right-16 top-16 text-2xl">
                        <FontAwesomeIcon icon={faXmark} />
                    </button>

                    <h2 className="text-3xl mb-3">Create new project</h2>

                    <label className="text-xl mt-3" htmlFor="title">Project title</label>
                    <input className="h-10 px-3 rounded-lg bg-dark-gray" type="text" name="title" />

                    <label className="text-xl mt-3" htmlFor="visibility">Visibility</label>
                    <select className="h-10 px-3 rounded-lg bg-dark-gray" name="visibility">
                        <option value="public">Public</option>
                        <option value="private">Private</option>
                    </select>

                    <button type="submit" className="outline-2 rounded-md text-2xl p-5 mt-10 hover:bg-white hover:text-blue-gray">
                        <FontAwesomeIcon className="mr-2" icon={faPlus} />
                        Create project
                    </button>
                </form>
            </div>
        }
    </>;
}

export default Dashboard;