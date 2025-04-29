export interface InlineUser {
    username: string,
    pictureUrl: string,
}

export interface User extends InlineUser {
    joinDate: string,
    bio: string,
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
    public: boolean,
}

export interface ProjectComment {
    user: InlineUser,
    contents: string,
    children: ProjectComment[],
}