import { useEffect } from 'react';
import GithubLogo from '../assets/github.png';
import { useAuth } from '../auth.tsx';
import { useNavigate } from 'react-router';

function SignIn() {
  const clientId = import.meta.env.VITE_GITHUB_CLIENT_ID;
  const url = "https://github.com/login/oauth/authorize?client_id=" + clientId;

  const auth = useAuth();
  const navigate = useNavigate();

  useEffect(() => {
      if (auth.isAuth) {
          navigate("/profile");
      }
  }, [auth]);

  return (
    <div className="flex justify-center items-center">
      <div className="flex-row bg-light-gray text-black rounded-4xl max-w-[95vw] p-8">
        <h3 className="font-normal self-left text-3xl">Sign-in</h3>
        <a href={url} className="flex bg-github-gray rounded-xl mx-8 py-1 px-10 items-center my-14">
          <img src={GithubLogo} className="size-8 m-2 mx-2" />
          <span className="text-white text-2xl font-medium">Sign-in with Github</span>
        </a>
      </div>
    </div>
  )
}

export default SignIn;
