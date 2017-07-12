import React, { Component } from 'react';
import DocumentTitle from 'react-document-title';
import Highlight from 'react-fast-highlight';

import axios from 'axios';

import Paths from './Paths';

class Blob extends Component {
  constructor(props) {
    super(props);
    this.state = {
      doc: '',
    }
  }
  componentDidMount() {
    let that = this;
    axios.get('/ds/cat/' + this.props.match.params.id).then(resp => {
      that.setState({
        doc: resp.data
      });
    })
  }

  render() {
    return (
      <DocumentTitle title={'DXR: contents: ' + this.props.match.params.id}>
        <div>
          <h2>Names</h2>
          <Paths id={this.props.match.params.id}/>

          <h2>Contents</h2>
          <Highlight languages={['xml', 'perl', 'cpp']}>
            {this.state.doc}
          </Highlight>
        </div>
      </DocumentTitle>
    );
  }
}

export default Blob;
