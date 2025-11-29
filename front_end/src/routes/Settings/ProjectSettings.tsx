import { useNavigate, useParams } from "react-router";
import { fetchApi, useApi } from "../../utils";
import { Project } from "../../types";
import Loading from "../../components/Loading";
import Button from "../../components/Button";
import { FormEvent, useEffect, useState } from "react";
import TagInput from "./TagInput";

function ProjectSettings() {
    const params = useParams();
    const [project,] = useApi<Project>(`/project/${params.username}/${params.id}`)

    const navigate = useNavigate();

    async function saveSettings(e: FormEvent<HTMLFormElement>) {
        e.preventDefault();

        const projectData = new FormData(e.target as HTMLFormElement);

        const title = projectData.get("title") as string;
        const visibility = projectData.get("visibility") as string;

        await fetchApi(`/project/${params.username}/${params.id}`, {
            method: "PUT",
            headers: {
                "Content-Type": "application/json",
            },
            body: JSON.stringify({ title, private: visibility == "private", tags }),
        });
    }

    useEffect(() => {
        if (project !== undefined) {
            setTags(project.tags);
        }
    }, [project]);

    const [tags, setTags] = useState<string[]>([]);

    const inputStyle = "h-10 pl-3 pr-5 mb-5 rounded-lg bg-dark-gray border border-white w-full outline-none";

    return (
        <div className="px-24 py-8">
            <h2 className="text-3xl font-medium mb-10">Project Settings</h2>
            {project === undefined ? (
                <Loading />
            ) : (
                <>
                    <form onSubmit={saveSettings}>
                        <label className="block text-xl mb-2" htmlFor="title">
                            Title
                        </label>
                        <input
                            className={inputStyle}
                            type="text"
                            name="title"
                            id="title"
                            defaultValue={project.title}
                        />
                        <label className="block text-xl mb-2" htmlFor="visibility">
                            Visibility
                        </label>
                        <select
                            className={inputStyle}
                            name="visibility"
                            id="visibility"
                            defaultValue={project.public ? "public" : "private"}
                        >
                            <option value="public">Public</option>
                            <option value="private">Private</option>
                        </select>

                        <span className="block text-xl mb-2">Tags</span>
                        <TagInput className={inputStyle} tags={tags} setTags={setTags} initialTags={project.tags} />

                        <div className="my-5">
                            <Button>Save settings</Button>
                        </div>
                    </form>

                    <Button onClick={() => navigate(`/project/${params.username}/${params.id}`)}>
                        Back to project
                    </Button>
                </>
            )}
        </div>

    );
}

export default ProjectSettings;