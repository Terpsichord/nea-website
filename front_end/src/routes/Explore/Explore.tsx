import ProjectView from "../../components/ProjectView";
import { ProjectInfo } from "../../types";
import { useApi } from "../../utils";
import SearchBar from "./SearchBar";
import { useSearchParams } from "react-router";
import SearchPage from "./SearchPage";

function Explore() {
    const [params] = useSearchParams();
    const searchQuery = params.get("search");

    const [projects, error] = useApi<ProjectInfo[]>(`/projects`);

    return (
        <div className="space-y-6 px-24">
            <SearchBar />
            {searchQuery ?
                <SearchPage /> :
                <>
                    {projects && <ProjectView projects={projects} error={error} className="lg:grid-cols-2 grid-cols-1 gap-x-20 gap-y-14" />}
                </>
            }
        </div>
    );
}

export default Explore;