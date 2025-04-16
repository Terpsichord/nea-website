import { StrictMode } from 'react'
import ReactDOM from 'react-dom/client'
import { BrowserRouter, Routes, Route } from 'react-router';
import './index.css'
import AuthProvider from './auth.tsx';
import Navbar from './components/Navbar.tsx';
import Home from './routes/Home.tsx'
import SignIn from './routes/SignIn.tsx';
import Profile from './routes/Profile.tsx';
import User from './routes/User.tsx';
import ProjectPage from './routes/ProjectPage.tsx';

const root = document.getElementById('root')!;

ReactDOM.createRoot(root).render(
  <StrictMode>
    <BrowserRouter>
      <AuthProvider>
        <Navbar />
        <Routes>
          <Route path="/" element={<Home />} />
          <Route path="/signin" element={<SignIn />} />
          <Route path="/profile" element={ <Profile /> } />
          <Route path="/user/:username" element={<User />} />
          <Route path="/project/:username/:id" element={<ProjectPage />}/>
        </Routes>
      </AuthProvider>
    </BrowserRouter>
  </StrictMode>,
)

