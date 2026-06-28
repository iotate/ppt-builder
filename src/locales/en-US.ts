export default {
  common: {
    save: 'Save',
    cancel: 'Cancel',
    delete: 'Delete',
    edit: 'Edit',
    create: 'Create',
    confirm: 'Confirm',
    loading: 'Loading...',
    success: 'Success',
    error: 'Error',
    warning: 'Warning',
    search: 'Search',
    reset: 'Reset',
    upload: 'Upload',
    download: 'Download',
    open: 'Open',
    close: 'Close',
    back: 'Back',
    next: 'Next',
    previous: 'Previous',
    generate: 'Generate',
    regenerate: 'Regenerate',
    clear: 'Clear',
    copy: 'Copy',
    paste: 'Paste',
    refresh: 'Refresh'
  },
  menu: {
    projects: 'Projects',
    brainstorm: 'BrainStorm',
    outline: 'Outline',
    pages: 'PPTs',
    skeletons: 'Structures',
    templates: 'Templates',
    styles: 'Styles',
    api: 'API',
    logs: 'Logs',
    about: 'About'
  },
  project: {
    title: 'Projects',
    create: 'New Project',
    topic: 'Project Topic',
    topicPlaceholder: 'Enter topic, e.g., 2026 Annual Sales Report',
    open: 'Open',
    folder: 'Folder',
    deleteConfirm: 'Are you sure you want to delete this project?',
    deleteDesc: 'This will delete the project folder and all related files',
    createSuccess: 'Project structure will be initialized',
    createFailed: 'Failed to create',
    deleteSuccess: 'Deleted successfully',
    deleteFailed: 'Failed to delete',
    updatedAt: 'Updated at',
    status: {
      brainstorm: 'Brainstorm',
      outline: 'Outline',
      ppt: 'PPT'
    },
    noProjects: 'No projects yet, click the button above to create one'
  },
  brainstorm: {
    title: 'BrainStorm',
    subtitle: 'Clarify your presentation goals and key messages through conversation. AI will help organize your thoughts and generate requirements.',
    send: 'Send',
    generateRequirements: 'Generate Requirements',
    goToOutline: 'Go to Outline',
    backToProject: 'Back to Project',
    inputPlaceholder: 'Enter your questions or ideas...',
    thinking: 'Thinking...',
    emptyChat: 'Start the conversation! AI will help you clarify the core goals and key messages of your presentation.',
    user: 'You',
    assistant: 'AI Assistant',
    uploadAttachment: 'Upload Attachment',
    noConversation: 'Please have a conversation first',
    chatFailed: 'AI chat failed',
    generateFailed: 'Failed to generate requirements',
    attachments: 'Attachments',
    noAttachments: 'No attachments',
    openFolder: 'Open attachment folder'
  },
  outline: {
    title: 'Outline Plan',
    promptColumn: 'I Want',
    outlineColumn: 'Outline',
    style: 'Style',
    framework: 'Framework',
    freeStyle: 'FreeStyle',
    simple: 'Simple',
    medium: 'Standard',
    detailed: 'Detailed',
    infoDensity: 'Info Density',
    expectedPages: 'Expected Pages',
    generateOutline: 'Generate Outline',
    saveOutline: 'Save Outline',
    splitPages: 'Generate PPT Pages',
    backToBrainstorm: 'Back to Brainstorm',
    goToPages: 'Enter PPT Design',
    enterPrompt: 'Please enter your requirements',
    outlinePlaceholder: 'Generate or enter outline content...',
    pagesMustBePositive: 'Expected pages must be greater than 0',
    generateOrEnterOutline: 'Please generate or enter outline content first',
    confirmSplit: 'Split outline into pages?',
    saveFailed: 'Save failed',
    loadFailed: 'Load failed',
    defaultPrompt: `【Goal】Describe the core goal of the presentation
- Main purpose: Persuade/Educate/Report/Inspire
- Expected audience reaction or action
- Success criteria

【Audience】Describe the target audience
- Background and knowledge level
- Concerns and pain points
- Possible doubts

【Context】Describe the presentation scenario
- Occasion (Meeting/Speech/Remote)
- Time limit
- Brand or visual requirements`
  },
  pages: {
    title: 'PPT Designer',
    template: 'Template',
    backToOutline: 'Back to Outline',
    style: 'Style',
    freeStyle: 'FreeStyle',
    generateImage: 'Generate',
    regenerateImage: 'Regenerate',
    savePage: 'Save',
    previousPage: 'Previous',
    nextPage: 'Next',
    noImages: 'No images yet',
    generateAll: 'Generate All',
    exportPdf: 'Export PDF',
    exportPptx: 'Export PPT',
    exportEditablePptx: 'Export Editable PPT',
    exportEditablePptxEnhanced: 'Export Enhanced PPT',
    exportTemplatePptx: 'Export from Template',
    selectTemplateAndGenerateFirst: 'Please select template and generate images first',
    openFolderFailed: 'Failed to open folder',
    keepOnePage: 'At least one page must be kept',
    deletePageFailed: 'Failed to delete page',
    selectStyleFirst: 'Please select a style first',
    generateImageFailed: 'Image generation failed',
    checkErrorLog: 'See error log for details',
    batchGenerateFailed: 'Batch generation failed',
    generateImageFirst: 'Please generate image first',
    enterRefinePrompt: 'Please enter refinement requirements',
    refineFailed: 'Image refinement failed',
    exportFailed: 'Export failed',
    deleteCurrent: 'Delete Current',
    imagePreview: 'Image Preview',
    arrowKeyHint: 'Arrow keys to navigate',
    refine: 'Refine',
    openFolder: 'Open Folder'
  },
  skeleton: {
    title: 'Structure Management',
    create: 'New Structure',
    edit: 'Edit Structure',
    delete: 'Delete Structure',
    name: 'Structure Name',
    content: 'Structure Content',
    deleteConfirm: 'Are you sure you want to delete this structure?',
    noSkeletons: 'No structures yet, click the button above to create one',
    loadFailed: 'Failed to load structure content',
    enterName: 'Please enter structure name',
    saveFailed: 'Save failed',
    deleteFailed: 'Delete failed',
    pageDesc: 'A structure is a Markdown text describing the logical framework of a presentation, used to guide AI in generating outlines.',
    noDescription: 'No description',
    defaultTemplate: `# Structure Name

Briefly describe the purpose and applicable scenarios of this presentation structure.

## Core Logic

Describe the core logic flow of this structure, e.g.: A → B → C → D

## Applicable Scenarios

- Scenario 1
- Scenario 2
- Scenario 3

## Key Points

1. Step 1: Description
2. Step 2: Description
3. Step 3: Description

## Advantages

- Advantage 1
- Advantage 2

## Disadvantages

- Disadvantage 1
- Disadvantage 2

## Example

Provide a specific application example.
`
  },
  template: {
    title: 'Template Management',
    create: 'New Template',
    edit: 'Edit Template',
    delete: 'Delete Template',
    name: 'Template Name',
    description: 'Template Description',
    deleteConfirm: 'Are you sure you want to delete this template?',
    noTemplates: 'No templates yet, add templates in the templates folder',
    coverImage: 'Cover',
    contentImage: 'Content',
    backImage: 'Back',
    defaultCannotDelete: 'Default template cannot be deleted',
    deleteFailed: 'Delete failed',
    deleteNotImplemented: 'Delete template function not implemented yet',
    loadFailed: 'Failed to load templates',
    pageDesc: 'Templates contain cover, content page, and back cover images to guide AI in generating infographic styles and layouts.',
    none: 'None',
    howToAdd: 'How to Add Templates',
    helpText1: 'Create a new folder in ./templates/ to add a template. The folder name becomes the template name.',
    helpText2: 'Each template folder should contain the following files:',
    tip: 'Tip',
    tipDesc: 'After adding a template folder, click the Refresh button above to see the new template.'
  },
  style: {
    title: 'Style Management',
    create: 'New Style',
    edit: 'Edit Style',
    delete: 'Delete Style',
    name: 'Style Name',
    content: 'Style Description',
    deleteConfirm: 'Are you sure you want to delete this style?',
    noStyles: 'No styles yet, click the button above to create one',
    extractFromFile: 'Extract from File',
    extractFromTemplate: 'Extract from Template',
    selectTemplate: 'Select Template',
    extract: 'Extract Style',
    loadFailed: 'Failed to load style content',
    enterName: 'Please enter style name',
    saveFailed: 'Save failed',
    selectTemplateFirst: 'Please select a template',
    extractFailed: 'Failed to extract style',
    selectFileFirst: 'Please select a file first',
    deleteFailed: 'Delete failed',
    pageDesc: 'A style is a Markdown text that is passed to AI as part of the prompt when generating images, affecting colors, style, etc.',
    noColors: 'No color information',
    contentPlaceholder: 'Describe the characteristics of this style, including color schemes, design points, etc...',
    contentHint: 'Content will be part of the image generation prompt, recommend including color schemes, design styles, etc.',
    extractMode: 'Extraction Method',
    selectFile: 'Select File',
    selectFilePlaceholder: 'Select image or PPTX file',
    browse: 'Browse',
    fileHint: 'Supports PNG, JPG, WEBP, PPTX formats. PPTX files will automatically extract color schemes.',
    templateHint: 'After selecting a template, the template\'s color scheme and style will be automatically extracted',
    templatePreview: 'Template Preview',
    noPreview: 'No preview available',
    namePlaceholder: 'Style name will match template name',
    extractNote: 'Extraction Note',
    extractNoteFileDesc: 'Image extraction requires a multimodal LLM (e.g., GPT-4o, Qwen-VL). PPTX files will first extract color schemes, then generate style descriptions via LLM.',
    extractNoteTemplateDesc: 'Extracting from template analyzes the template\'s color scheme and design style to automatically generate a style description.',
    structureIncomplete: 'Style structure is incomplete, missing required sections:',
    saveAnyway: 'Save anyway?'
  },
  config: {
    title: 'API Configuration',
    description: 'Configure LLM and Image generation API for outline and image generation.',
    endpoint: 'API Endpoint',
    apiKey: 'API Key',
    model: 'Model Name',
    extraHeaders: 'Extra Headers',
    addHeader: 'Add Header',
    key: 'Key',
    value: 'Value',
    saveSuccess: 'Configuration saved successfully',
    saveFailed: 'Failed to save configuration',
    loadFailed: 'Failed to load configuration',
    connectionTest: 'Test Connection',
    testSuccess: 'Connection successful',
    testFailed: 'Connection failed',
    llmConfig: 'LLM Configuration (Outline Generation)',
    imgConfig: 'Image Generation Configuration',
    provider: 'Provider',
    customProvider: 'Custom',
    enterApiKey: 'Please enter API Key',
    enterEndpoint: 'Please enter API endpoint',
    llmTestSuccess: 'LLM API connected successfully! Configuration auto-saved',
    llmTestFailed: 'LLM API connection failed, please check configuration',
    imgTestSuccess: 'Image API connected successfully! Configuration auto-saved',
    imgTestFailed: 'Image API connection failed, please check configuration',
    customEndpointHint: 'Custom API endpoint, must be compatible with OpenAI API format',
    customImgEndpointHint: 'Custom image generation API endpoint',
    customHeadersHint: 'Add custom HTTP request headers',
    customHeadersHintOptional: 'Add custom HTTP request headers (if needed)',
    autoFillEditable: 'Auto-filled, editable',
    pleaseConfigure: 'Please configure API first'
  },
  logs: {
    title: 'Error Logs',
    clear: 'Clear Logs',
    clearConfirm: 'Are you sure you want to clear all logs?',
    noLogs: 'No error logs',
    timestamp: 'Time',
    message: 'Error Message',
    clearFailed: 'Failed to clear logs',
    loadFailed: 'Failed to load log file',
    description: 'View error.log file content for troubleshooting.',
    autoRefresh: 'Auto Refresh',
    stopAutoRefresh: 'Stop Auto Refresh',
    scrollToTop: 'Scroll to Top',
    scrollToBottom: 'Scroll to Bottom'
  },
  imageCard: {
    pageNum: 'Page {num}',
    pending: 'Pending',
    generating: 'Generating...',
    done: 'Done',
    failed: 'Failed',
    refine: 'Refine',
    regenerate: 'Regenerate'
  },
  promptEdit: {
    title: 'Edit Page {num} Prompt',
    label: 'Image Generation Prompt',
    placeholder: 'Describe the image content you want to generate...',
    tip: 'Image will be regenerated after modifying the prompt.',
    processing: 'Processing...',
    regenerate: 'Regenerate'
  },
  about: {
    title: 'About',
    description: 'An AI-powered structured PPT engine for professionals, supporting private models and brand asset accumulation.',
    coreValue: 'Core Value',
    coreValueDesc: 'We solve not the problem of "making PPT from scratch", but "the last mile from materials to professional presentation":',
    logicStructure: 'Logic Structuring',
    logicStructureDesc: 'AI acts as "Logic Architect", generating hierarchical outlines from user materials and goals',
    brandCompliance: 'Brand Compliance',
    brandComplianceDesc: 'AI acts as "Brand Compliance Gatekeeper", automatically conforming to corporate VI standards',
    dataPrivacy: 'Data Privacy',
    dataPrivacyDesc: 'Support BYOK (Bring Your Own Key), data flows through your API call chain',
    targetUsers: 'Target Users',
    targetUsersList: [
      'Corporate strategy/consulting advisors: producing large amounts of presentation materials daily',
      'Pre-sales solution experts: frequently producing dozens of pages of materials',
      'University professors/researchers: having lots of existing documents',
      'Financial analysts: requiring high data accuracy'
    ],
    differentiation: 'Differentiation',
    diffList: [
      'White-box structuring: define outline framework first, then generate content, fully controllable',
      'Deep master parsing: perfectly restore enterprise-exclusive PPTX masters',
      'Native high-fidelity export: 100% editable text boxes and native charts',
      'BYOK (Bring Your Own Key): data privacy, flexible model switching'
    ],
    features: 'Features',
    brainstorming: 'Brainstorming',
    brainstormingDesc: 'Clarify presentation goals through dialogue, upload text attachments, AI helps generate requirement documents',
    outlineGeneration: 'Outline Generation',
    outlineGenerationDesc: 'Smart outline generation, support for multiple presentation frameworks, info density options',
    pptDesign: 'PPT Design',
    pptDesignDesc: 'Batch image generation, image refinement, template reference, style customization',
    exportShare: 'Export & Share',
    exportShareDesc: 'PDF export, PPT export',
    customConfig: 'Custom Configuration',
    customConfigDesc: 'LLM API, template management, style management, framework management',
    contact: 'Contact',
    github: 'GitHub'
  }
}
