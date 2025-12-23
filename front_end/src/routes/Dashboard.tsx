import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import ProjectView from "../components/ProjectView";
import { faPlus, faXmark } from "@fortawesome/free-solid-svg-icons";
import { FormEvent, useState } from "react";
import { fetchApi, useApi } from "../utils";
import { ProjectInfo } from "../types";

function Dashboard() {
    const [showModal, setShowModal] = useState(false);

    async function createProject(e: FormEvent<HTMLFormElement>) {
        e.preventDefault();

        const projectData = new FormData(e.target as HTMLFormElement);
        const title = projectData.get("title") as string;
        const visibility = projectData.get("visibility") as string;
        const lang = projectData.get("lang") as string;

        const response = await fetchApi("/project/new", {
            method: "POST",
            headers: {
                "Content-Type": "application/json",
            },
            body: JSON.stringify({ title, lang, private: visibility == "private" })
        });

        console.log({ response });
        if (response.ok) {
            const { username, repoName } = await response.json();
            window.location.href = `/editor/${username}/${repoName}`
        } else {
            // TODO: error handling
        }
    };
    
    const [projects, error] = useApi<ProjectInfo[]>("/profile/projects");

    return <>
        <div className="container mx-auto">
            <div className="flex justify-center items-center">
                <button onClick={() => setShowModal(true)} className="outline-2 rounded-md text-2xl p-5 my-32 hover:bg-white hover:text-black">
                    <FontAwesomeIcon className="mr-2" icon={faPlus} />
                    Create new project
                </button>
            </div>
            <h2 className="text-4xl mb-5">Your projects</h2>
            <ProjectView projects={projects} error={error} className="grid grid-flow-row gap-x-10" />
        </div>
        {
            showModal &&
            <div className="fixed top-0 left-0 w-full h-full bg-black/45">
                <form onSubmit={createProject} className="flex flex-col py-8 px-10 rounded-3xl m-10 bg-blue-gray">
                    <button onClick={() => setShowModal(false)} className="absolute right-16 top-16 text-2xl">
                        <FontAwesomeIcon icon={faXmark} className="cursor-pointer" />
                    </button>

                    <h2 className="text-3xl mb-3">Create new project</h2>

                    <label className="text-xl mt-3" htmlFor="title">Project title</label>
                    <input className="h-10 px-3 rounded-lg bg-dark-gray" type="text" name="title" />

                    <label className="text-xl mt-3" htmlFor="lang">Language</label>
                    <select className="h-10 px-3 rounded-lg bg-dark-gray" name="lang">
                        <option value="py">Python</option>
                        <option value="js">JavaScript</option>
                        <option value="ts">TypeScript</option>
                        <option value="rs">Rust</option>
                        <option value="c">C</option>
                        <option value="cpp">C++</option>
                        <option value="cs">C#</option>
                        <option value="sh">Bash</option>
                        <option value="java">Java</option>
                    </select>

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