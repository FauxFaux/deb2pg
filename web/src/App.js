import React, { Component } from 'react';
import DocumentTitle from 'react-document-title';

import './App.css';

class App extends Component {
  render() {
    return (
      <DocumentTitle title='DXR'>
        <span>Hello, world!</span>
      </DocumentTitle>
    );
  }
}

export default App;
