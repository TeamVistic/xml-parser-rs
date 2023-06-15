use std::collections::HashMap;

#[derive(Debug)]
pub enum XMLElement<'a> {
    Element(
        &'a str,
        HashMap<&'a str, Vec<&'a str>>,
        Vec<&'a str>,
        Vec<XMLElement<'a>>,
    ),
    EmptyElement(&'a str, HashMap<&'a str, Vec<&'a str>>),
    Comment(&'a str),
    Cdata(&'a str),
}

enum XMLParsingSection<'a> {
    ElementStart(&'a str, HashMap<&'a str, Vec<&'a str>>),
    ElementStop(&'a str),
    FinishedElement(XMLElement<'a>),
    EmptyElement(XMLElement<'a>),
    Comment(XMLElement<'a>),
    Cdata(XMLElement<'a>),
    Content(&'a str),
}

fn parse_element_name_and_attributes(raw_xml: &str) -> (&str, HashMap<&str, Vec<&str>>) {
    if let Some((name, raw_attributes)) = raw_xml.split_once(' ')
    // removes the pre- and suffix as well as split the tag into the name and the attribute list: <name attribute_one="one two" attribute_two="one two"> -> name & attribute_one="one two" attribute_two="one two"
    {
        let mut attributes: HashMap<&str, Vec<&str>> = HashMap::<&str, Vec<&str>>::new();
        for attribute_pair in raw_attributes.split("\" ").collect::<Vec<&str>>() {
            // splits the attribute list into name and value pairs: attribute_one="one two" attribute_two="one two" -> attribute_one="one two & attribute_two="one two"
            let (name, mut values) = attribute_pair.split_once("=\"").unwrap();
            if let Some(stripped_values) = values.strip_suffix('\"') {
                // the last one will have one final quotation mark
                values = stripped_values;
            }
            attributes.insert(
                name,
                values.split(" ").collect::<Vec<&str>>(), // converts the value list into a vector: one two -> [one, two]
            );
        }
        return (name, attributes);
    }
    (
        raw_xml, // if the stripped_xml does not contain a whitespace, it is the name of the element and there are no attributes
        HashMap::<&str, Vec<&str>>::new(),
    )
}
fn parse_version(raw_xml: &str) -> XMLParsingSection {
    let stripped_xml = raw_xml
        .strip_prefix("<?")
        .unwrap()
        .strip_suffix("?>")
        .unwrap();
    let (name, attributes) = parse_element_name_and_attributes(stripped_xml);
    XMLParsingSection::EmptyElement(XMLElement::EmptyElement(name, attributes))
}
fn parse_element_start_tag(raw_xml: &str) -> XMLParsingSection {
    let stripped_xml = raw_xml
        .strip_prefix("<")
        .unwrap()
        .strip_suffix(">")
        .unwrap();
    let (name, attributes) = parse_element_name_and_attributes(stripped_xml);
    XMLParsingSection::ElementStart(name, attributes)
}
fn parse_element_stop_tag(raw_xml: &str) -> XMLParsingSection {
    XMLParsingSection::ElementStop(
        raw_xml
            .strip_prefix("</")
            .unwrap()
            .strip_suffix(">")
            .unwrap(),
    )
    // remove the pre- and suffix of the end-tag: </name> -> name
}
fn parse_empty_element_tag(raw_xml: &str) -> XMLParsingSection {
    let stripped_xml = raw_xml
        .strip_prefix("<")
        .unwrap()
        .strip_suffix("/>")
        .unwrap();
    let (name, attributes) = parse_element_name_and_attributes(stripped_xml);
    XMLParsingSection::EmptyElement(XMLElement::EmptyElement(name, attributes))
}
fn parse_comment(raw_xml: &str) -> XMLParsingSection {
    XMLParsingSection::Comment(XMLElement::Comment(
        raw_xml
            .strip_prefix("<!-- ")
            .unwrap()
            .strip_suffix(" -->")
            .unwrap(),
    ))
    // remove the pre- and suffix of the end-tag: <!-- comment --> ->  comment
}
fn parse_cdata(raw_xml: &str) -> XMLParsingSection {
    XMLParsingSection::Cdata(XMLElement::Cdata(
        raw_xml
            .strip_prefix("<![CDATA[")
            .unwrap()
            .strip_suffix("]]>")
            .unwrap(),
    ))
    // remove the pre- and suffix of the end-tag: <![CDATA[cdata]]> ->  cdata
}

pub fn parse(raw_xml: &str) -> Vec<XMLElement> {
    let mut result = Vec::<XMLElement>::new();
    let mut section_stack = Vec::<XMLParsingSection>::new();
    for mut section in raw_xml.split_inclusive('>').collect::<Vec<&str>>() {
        if !section.starts_with('<') {
            if let Some(index) = section.find('<') {
                let (content, update_section) = section.split_at(index);
                section = update_section;
                if !content.chars().all(|x| x == '\n' || x == ' ') {
                    // if the section is only newlines or spaces, it can be omitted
                    section_stack.push(XMLParsingSection::Content(content));
                }
            } else if !section.chars().all(|x| x == '\n' || x == ' ') {
                // if the section is only newlines or spaces, it can be omitted
                section_stack.push(XMLParsingSection::Content(section));
            }
        }
        if section.ends_with("/>") {
            // empty-element tag
            if section_stack.is_empty() {
                // there is currently no parent element
                if let XMLParsingSection::EmptyElement(element) = parse_empty_element_tag(section) {
                    result.push(element);
                }
            } else {
                section_stack.push(parse_empty_element_tag(section));
            }
        } else if section.starts_with("</") {
            // end-tag
            if let XMLParsingSection::ElementStop(parent_name) = parse_element_stop_tag(section) {
                let mut contents = Vec::<&str>::new();
                let mut children = Vec::<XMLElement>::new();
                loop {
                    if let Some(section) = section_stack.pop() {
                        match section {
                            XMLParsingSection::ElementStart(name, attributes) => {
                                if name == parent_name {
                                    // the start tag of the stop tag was found -> end the parsing of this element
                                    children.reverse(); // as they are added in reverse order, they have to be inversed again
                                    section_stack.push(XMLParsingSection::FinishedElement(
                                        XMLElement::Element(name, attributes, contents, children),
                                    ));
                                    break;
                                }
                            }
                            XMLParsingSection::ElementStop(_) => {
                                // this should never happen
                            }
                            XMLParsingSection::FinishedElement(element) => {
                                children.push(element);
                            }
                            XMLParsingSection::EmptyElement(element) => {
                                children.push(element);
                            }
                            XMLParsingSection::Comment(element) => {
                                children.push(element);
                            }
                            XMLParsingSection::Cdata(element) => {
                                children.push(element);
                            }
                            XMLParsingSection::Content(content) => {
                                contents.push(content);
                            }
                        }
                    }
                }
            }
        } else if section.starts_with("<?") {
            // start-tag
            if let XMLParsingSection::EmptyElement(element) = parse_version(section) {
                result.push(element)
            }
        } else if section.starts_with('<') {
            // start-tag
            section_stack.push(parse_element_start_tag(section)); // always push to stack to make it the current parent element
        } else if section.starts_with("<!--") {
            // comment
            if section_stack.is_empty() {
                // there is currently no parent element
                if let XMLParsingSection::Comment(element) = parse_comment(section) {
                    result.push(element);
                }
            } else {
                section_stack.push(parse_comment(section));
            }
        } else if section.starts_with("<![CDATA[") {
            // CDATA
            if section_stack.is_empty() {
                // there is currently no parent element
                if let XMLParsingSection::Cdata(element) = parse_cdata(section) {
                    result.push(element);
                }
            } else {
                section_stack.push(parse_cdata(section));
            }
        }
    }
    for element in section_stack {
        // adding any remaining elements to the result
        match element {
            XMLParsingSection::FinishedElement(element) => result.push(element),
            _ => {}
        }
    }
    result
}
