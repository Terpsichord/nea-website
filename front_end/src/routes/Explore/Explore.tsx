import ProjectView from "../../components/ProjectView";
import { Category } from "../../types";
import { useApi } from "../../utils";
import SearchBar from "./SearchBar";
import { useSearchParams } from "react-router";
import SearchPage from "./SearchPage";

function Explore() {
    const [params] = useSearchParams();
    const searchQuery = params.get("search");

    const [recCategories, error] = useApi<Category[]>("/rec");

    return (
        <div className="space-y-6 px-24">
            <SearchBar />
            {searchQuery ?
                <SearchPage /> :
                <>
                    {recCategories &&
                        <div>{
                            recCategories.map(cat => 
                                <div>
                                    <h3>{cat.name}</h3>
                                    <ProjectView projects={cat.projects} error={error} className="lg:grid-cols-2 grid-cols-1 gap-x-20 gap-y-14" />
                                </div>
                            )
                        }</div>
                    }
                </>
            }
        </div>
    );
}

export default Explore;