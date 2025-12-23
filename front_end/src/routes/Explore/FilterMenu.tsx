import { FormEvent, useRef, useState } from "react";
import ContextMenu from "../../components/ContextMenu";
import Button from "../../components/Button";
import { useSearchParams } from "react-router";

function FilterMenu() {
    const [showMenu, setShowMenu] = useState(false);
    const [_params, setParams] = useSearchParams();
    const menuParent = useRef<HTMLDivElement | null>(null);

    function applyFilters(e: FormEvent<HTMLFormElement>) {
        e.preventDefault();

        const filters = new FormData(e.target as HTMLFormElement);

        const sort = filters.get("sort") as string;
        const dir = filters.get("dir") as string;
        const lang = filters.get("lang") as string;
        const tagText = filters.get("tags") as string;

        const tags = tagText.split("\n").map(tag => tag.trim()).map(tag => ["tags", tag]);

        setParams(prev => {
            const search = prev.get("search")!;

            let params = new URLSearchParams([["search", search], ...tags]);
            if (sort) params.set("sort", sort);
            if (dir) params.set("dir", dir);
            if (lang) params.set("lang", lang);

            return params;
        });
    };

    return (
        <div className="ml-auto bg-blue-gray text-lg p-2" ref={menuParent} onClick={() => setShowMenu(true)}>
            Filter
            {showMenu &&
                <ContextMenu parent={menuParent} setShow={setShowMenu}>
                    <form className="flex flex-col" onSubmit={applyFilters}>
                        <label>
                            Sort by
                        </label>
                        <select name="sort" className="rounded-lg bg-dark-gray">
                            <option value="relevant">Relevance</option>
                            <option value="title">Title</option>
                            <option value="likes">Likes</option>
                            <option value="upload_time">Upload Time</option>
                        </select>
                        <label>
                            Order
                        </label>
                        <select name="dir" className="rounded-lg bg-dark-gray">
                            <option value="asc">Ascending</option>
                            <option value="desc">Descending</option>
                        </select>
                        <div>
                            Filter by
                        </div>
                        <select name="lang" className="rounded-lg bg-dark-gray">
                            <option value="">Any</option>
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
                        <div>
                            Tags
                        </div>
                        <textarea name="tags" />
                        <div className="mt-3">
                            <Button>Apply</Button>
                        </div>
                    </form>
                </ContextMenu>
            }
        </div>
    )
}

export default FilterMenu;