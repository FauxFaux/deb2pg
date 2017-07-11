import React, { Component } from 'react';
import Highlight from 'react-fast-highlight';
import axios from 'axios';

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
      <Highlight languages={['c', 'xml', 'perl']}>
        {this.state.doc}
      </Highlight>
    );
  }
}

export default Blob;
