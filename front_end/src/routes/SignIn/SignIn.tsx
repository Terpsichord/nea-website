import { useEffect } from 'react';
import { useAuth } from '../../auth.tsx';
import { useNavigate } from 'react-router';
import GithubButton from './GithubButton.tsx';
import AccentBox from '../../components/AccentBox.tsx';

function SignIn() {
  const auth = useAuth();
  const navigate = useNavigate();

  useEffect(() => {
    if (auth.isAuth) {
      navigate("/profile");
    }
  }, [auth]);

  return (
    <div className="container mx-auto">
    <AccentBox size="md">
      <h3 className="font-normal self-left text-3xl">Sign-in</h3>
      <div className="flex justify-center items-center px-8 pt-14 pb-10">
        <GithubButton />
      </div>
    </AccentBox>
    </div>

  )
}

export default SignIn;
