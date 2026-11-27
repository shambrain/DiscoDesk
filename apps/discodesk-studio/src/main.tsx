import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import App from './App';
import './styles.css';

const container = document.getElementById('root');
if (container === null) {
  // A missing root element means the HTML and the bundle disagree; failing
  // loudly is better than rendering nothing and looking like a hang.
  throw new Error('the Studio root element is missing from index.html');
}

createRoot(container).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
