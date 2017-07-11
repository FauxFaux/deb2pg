import React from 'react';
import ReactDOM from 'react-dom';
import { HashRouter, Route } from 'react-router-dom'

import './index.css';
import './highlight-tomorrow.css';
import App from './App';
import Blob from './Blob';
import registerServiceWorker from './registerServiceWorker';

ReactDOM.render(
  <HashRouter>
    <div>
      <Route exact path="/" component={App}/>
      <Route path="/blob/:id" component={Blob}/>
    </div>
  </HashRouter>, document.getElementById('root'));

registerServiceWorker();
