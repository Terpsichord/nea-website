import { faMagnifyingGlass } from "@fortawesome/free-solid-svg-icons";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { useState } from "react";
import { useSearchParams } from "react-router";

function SearchBar() {
    const [params, setParams] = useSearchParams();
    const [query, setQuery] = useState("");

    function search() {
        if (query.trim()) {
            params.set("search", query);
        } else {
            params.delete("search");
        }

        setParams(params);
    }

    return (
        <form action={search} className="relative">
            <FontAwesomeIcon icon={faMagnifyingGlass} className="absolute left-0 pl-2 transform top-1/2 -translate-y-1/2" />
            <input
                name="query"
                type="text"
                value={query}
                onChange={e => setQuery(e.target.value)}
                placeholder="Search"
                className="w-full h-10 pl-8 pr-2 border-2 border-gray rounded-3xl outline-none"
            />
        </form>
    );
}

export default SearchBar;