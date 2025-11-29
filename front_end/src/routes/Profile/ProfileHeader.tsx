function ProfileHeader({ follows }: { follows?: boolean }) {
    return (
        <div className="flex justify-between items-center mb-4">
            <h2 className="text-2xl">
                <a href="#" className={follows ? "" : "font-bold"}>Info</a>
                {" | "}
                <a href="#follows" className={follows ? "font-bold" : ""}>Follows</a>
            </h2>
        </div>
    );
}

export default ProfileHeader;