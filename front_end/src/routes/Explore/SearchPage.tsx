// import { useSearchParams } from "react-router";
import FilterMenu from "./FilterMenu";
// import { useApi } from "../../utils";
// import { ProjectInfo } from "../../types";

function SearchPage() {
    // const [params] = useSearchParams();

    // const query = params.get("search")!; 
    // const sort = params.get("sort");
    // const tags = params.get("tags");

    // const [projects, error] = useApi<ProjectInfo[]>(`/project/${query}&sort=${sort}&tags=${tags}`, { deps: [query, sort, tags] });

    return (
        <div className="flex justify-center items-center relative w-full">
            <div className="mt-6 ml-auto">
                <FilterMenu />
            </div>
        </div>
    )
}

export default SearchPage;