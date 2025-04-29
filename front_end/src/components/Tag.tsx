function Tag({ contents }: { contents: string }) {
    return (
        // TODO: add triangle tag shape (wip https://jsfiddle.net/zcdLjmsf/)
        <div className="inline bg-light-gray text-black px-1 py-0.5">{contents}</div>
    )
}

export default Tag;