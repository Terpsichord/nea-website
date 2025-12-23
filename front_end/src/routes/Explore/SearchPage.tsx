import { useNavigate, useSearchParams } from "react-router";
import FilterMenu from "./FilterMenu";
import { useApi } from "../../utils";
import { ProjectInfo } from "../../types";
import ProjectView from "../../components/ProjectView";

function SearchPage() {
    const [params] = useSearchParams();

    const query = params.get("search")!; 
    const sort = params.get("sort");
    const dir = params.get("dir");
    const lang = params.get("lang");
    const tags = params.getAll("tags");

    const navigate = useNavigate();
    if (!query) {
        navigate("/explore");
    }

    const sortText = sort ? `&sort=${sort}` : "";
    const langText = lang ? `&lang=${lang}` : "";
    const dirText = dir ? `&dir=${dir}` : "";
    const tagsText = tags.map(tag => `&tags=${tag}`).join("");

    const [projects, error] = useApi<ProjectInfo[]>(`/project/search?query=${query}${tagsText}${sortText}${dirText}${langText}`, { deps: [query, sort, dir, lang, tags] });

    return (
        <div className="flex flex-col justify-center items-center relative w-full">
            <div className="ml-auto mb-10">
                <FilterMenu />
            </div>
            {projects && <ProjectView projects={projects} error={error} className="lg:grid-cols-2 grid-cols-1 gap-x-20 gap-y-14" />}
        </div>
    )
}

export default SearchPage;