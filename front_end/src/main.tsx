import { StrictMode } from 'react'
import ReactDOM from 'react-dom/client'
import { BrowserRouter, Routes, Route } from 'react-router';
import './index.css'
import AuthProvider from './auth.tsx';
import Navbar from './components/Navbar.tsx';
import Home from './routes/Home.tsx'
import SignIn from './routes/SignIn/SignIn.tsx';
import Profile from './routes/Profile/Profile.tsx';
import User from './routes/User/User.tsx';
import ProjectPage from './routes/Project/ProjectPage.tsx';
import ProjectSettings from './routes/Settings/ProjectSettings.tsx';
import Explore from './routes/Explore/Explore.tsx';
import About from './routes/About.tsx';

const root = document.getElementById('root')!;

ReactDOM.createRoot(root).render(
  <StrictMode>
    <BrowserRouter>
      <AuthProvider>
        <Navbar />
        <Routes>
          <Route path="/" element={<Home />} />
          <Route path="/explore" element={<Explore />} />
          <Route path="/about" element={<About />} />
          <Route path="/signin" element={<SignIn />} />
          <Route path="/profile" element={ <Profile /> } />
          <Route path="/user/:username" element={<User />} />
          <Route path="/project/:username/:id" element={<ProjectPage />}/>
          <Route path="/project/:username/:id/settings" element={<ProjectSettings />}/>
        </Routes>
      </AuthProvider>
    </BrowserRouter>
  </StrictMode>,
)

