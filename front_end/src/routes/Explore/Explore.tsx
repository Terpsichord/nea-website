import ProjectView from "../../components/ProjectView";
import { ProjectInfo } from "../../types";
import { useApi } from "../../utils";
import SearchBar from "./SearchBar";
import { useSearchParams } from "react-router";
import SearchPage from "./SearchPage";

function Explore() {
    const [params] = useSearchParams();
    const searchQuery = params.get("search");

    const [projects, error] = useApi<ProjectInfo[]>(`/profile/projects`);

    return (
        <div className="px-24">
            <SearchBar />
            {searchQuery ?
                <SearchPage /> :
                <>
                    {projects && <ProjectView projects={projects} error={error} className="grid grid-flow-row gap-x-10" />}
                </>
            }
        </div>
    );
}

export default Explore;