export interface User {
    username: string
    joinDate: string,
    bio: string,
    pictureUrl: string,
}

export interface ProjectInfo {
    title: string,
    username: string,
    pictureUrl: string,
    repoName: string,
    tags: string[],
    readme: string,
    likeCount: number,
}

export interface Project extends ProjectInfo {
    githubUrl: string,
    uploadTime: string,
}