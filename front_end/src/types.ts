export interface User {
    username: string
    joinDate: string,
    bio: string,
    pictureUrl: string,
}

export interface ProjectInfo {
    title: string,
    tags: string[],
    readme: string,
}