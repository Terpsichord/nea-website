import GithubLogo from '../../assets/github.png';

function GithubButton() {
  const clientId = import.meta.env.VITE_GITHUB_CLIENT_ID;
  const url = "https://github.com/login/oauth/authorize?client_id=" + clientId;

    return (
        <a href={url} className="flex items-center bg-github-gray w-[340px] rounded-xl px-10 py-1">
          <img src={GithubLogo} className="size-8 m-2" />
          <span className="text-white text-2xl font-medium">Sign-in with Github</span>
        </a>
    );
}

export default GithubButton;